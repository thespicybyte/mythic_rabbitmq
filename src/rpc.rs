// RPC queue listener logic.
// Mirrors ReceiveFromRPCQueue in rabbitmq/utils.go and the individual
// recv_c2_rpc_*.go files from MythicMeta/MythicContainer.
//
// Each C2 RPC handler is registered as a tokio task that:
//   1. Declares a durable queue bound to `mythic_exchange` with the namespaced
//      routing key (e.g. "reverse_tcp_c2_rpc_config_check").
//   2. Consumes messages with manual ack.
//   3. Calls the user-supplied handler function with the deserialized request.
//   4. Publishes the serialized response back to delivery.reply_to with the
//      matching correlation_id.
//   5. Acks the delivery.
//   6. On any error, reconnects and retries.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::process::Command;

use lapin::{
    BasicProperties, Connection,
    options::{BasicAckOptions, BasicConsumeOptions, BasicNackOptions, BasicPublishOptions},
    types::FieldTable,
};
use serde::{Serialize, de::DeserializeOwned};
use tracing::{debug, error, info, warn};

use crate::connection::{connect_with_retry, declare_and_bind_queue, declare_mythic_exchange, open_channel, RabbitMQConfig};
use crate::constants::{C2_RPC_RESYNC_ROUTING_KEY, C2_RPC_START_SERVER_ROUTING_KEY, C2_RPC_STOP_SERVER_ROUTING_KEY, MYTHIC_EXCHANGE, RETRY_CONNECT_DELAY_SECS, get_routing_key};
use crate::error::{MythicError, Result};
use crate::structs::{C2ProfileDefinition, C2Parameter};

/// Spawn a long-running tokio task that listens on one RPC queue and dispatches
/// incoming messages to `handler`.
///
/// `Req` - the request type (must be Deserializable from JSON)
/// `Resp` - the response type (must be Serializable to JSON)
/// `handler` - a sync function `fn(Req) -> Resp`
///
/// The task reconnects automatically if the connection or channel drops.
pub fn spawn_rpc_listener<Req, Resp, F>(
    config: RabbitMQConfig,
    routing_key: String,
    handler: F,
) where
    Req: DeserializeOwned + Send + 'static,
    Resp: Serialize + Send + 'static,
    F: Fn(Req) -> Resp + Send + Sync + 'static,
{
    tokio::spawn(async move {
        loop {
            let conn = connect_with_retry(&config).await;
            if let Err(e) = run_rpc_loop::<Req, Resp, F>(&conn, &routing_key, &handler).await {
                error!(
                    "RPC listener for '{}' encountered an error: {}. Reconnecting in {}s...",
                    routing_key, e, RETRY_CONNECT_DELAY_SECS
                );
                tokio::time::sleep(Duration::from_secs(RETRY_CONNECT_DELAY_SECS)).await;
            }
        }
    });
}

/// Inner loop: set up the channel and consume until an error occurs.
async fn run_rpc_loop<Req, Resp, F>(
    conn: &Connection,
    routing_key: &str,
    handler: &F,
) -> Result<()>
where
    Req: DeserializeOwned,
    Resp: Serialize,
    F: Fn(Req) -> Resp,
{
    let channel = open_channel(conn).await?;
    declare_mythic_exchange(&channel, MYTHIC_EXCHANGE).await?;
    declare_and_bind_queue(&channel, MYTHIC_EXCHANGE, routing_key).await?;

    info!("RPC listener ready on queue '{}'", routing_key);

    let mut consumer = channel
        .basic_consume(
            routing_key,
            &format!("{}_consumer", routing_key),
            BasicConsumeOptions {
                no_ack: false,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .map_err(MythicError::from)?;

    while let Some(delivery_result) = futures_lite::StreamExt::next(&mut consumer).await {
        let delivery = match delivery_result {
            Ok(d) => d,
            Err(e) => {
                error!("Error receiving delivery on '{}': {}", routing_key, e);
                return Err(MythicError::from(e));
            }
        };

        debug!("Received RPC message on '{}'", routing_key);

        // Deserialize request
        let request: Req = match serde_json::from_slice(&delivery.data) {
            Ok(r) => r,
            Err(e) => {
                warn!("Failed to deserialize request on '{}': {}", routing_key, e);
                // Nack without requeue so Mythic gets an error response
                let _ = delivery.nack(BasicNackOptions { requeue: false, ..Default::default() }).await;
                continue;
            }
        };

        // Call the user handler
        let response = handler(request);

        // Serialize response
        let response_bytes = match serde_json::to_vec(&response) {
            Ok(b) => b,
            Err(e) => {
                error!("Failed to serialize response on '{}': {}", routing_key, e);
                let _ = delivery.nack(BasicNackOptions { requeue: false, ..Default::default() }).await;
                continue;
            }
        };

        // Publish response back to reply_to with matching correlation_id
        if let Some(reply_to) = delivery.properties.reply_to() {
            let props = if let Some(correlation_id) = delivery.properties.correlation_id() {
                BasicProperties::default().with_correlation_id(correlation_id.clone())
            } else {
                BasicProperties::default()
            };

            if let Err(e) = channel
                .basic_publish(
                    "",                  // default exchange for direct reply
                    reply_to.as_str(),
                    BasicPublishOptions::default(),
                    &response_bytes,
                    props.with_content_type("application/json".into()),
                )
                .await
            {
                error!("Failed to publish RPC response on '{}': {}", routing_key, e);
                let _ = delivery.nack(BasicNackOptions { requeue: true, ..Default::default() }).await;
                continue;
            }
        } else {
            warn!("No reply_to on delivery for '{}', dropping response", routing_key);
        }

        // Ack after successful response send
        if let Err(e) = delivery.ack(BasicAckOptions::default()).await {
            error!("Failed to ack delivery on '{}': {}", routing_key, e);
            return Err(MythicError::from(e));
        }
    }

    Err(MythicError::RpcError(format!("Consumer stream for '{}' ended unexpectedly", routing_key)))
}

// ---------------------------------------------------------------------------
// Resync listener
// ---------------------------------------------------------------------------

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
struct C2RPCReSyncMessage {
    #[serde(rename = "c2_profile_name")]
    pub _name: String,
}

#[derive(Debug, serde::Serialize)]
struct C2RPCReSyncMessageResponse {
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
}

/// Listen on the resync queue. When Mythic asks us to resync, re-send the
/// full sync message. Mirrors C2RPCReSync in recv_c2_rpc_resync.go.
pub async fn run_resync_listener(
    config: RabbitMQConfig,
    c2_name: String,
    profile: C2ProfileDefinition,
    parameters: Vec<C2Parameter>,
) {
    let routing_key = get_routing_key(&c2_name, C2_RPC_RESYNC_ROUTING_KEY);

    loop {
        let conn = connect_with_retry(&config).await;
        if let Err(e) = run_resync_loop(&conn, &routing_key, &c2_name, &profile, &parameters).await {
            error!("Resync listener error: {}. Reconnecting in {}s...", e, RETRY_CONNECT_DELAY_SECS);
            tokio::time::sleep(Duration::from_secs(RETRY_CONNECT_DELAY_SECS)).await;
        }
    }
}

async fn run_resync_loop(
    conn: &Connection,
    routing_key: &str,
    c2_name: &str,
    profile: &C2ProfileDefinition,
    parameters: &[C2Parameter],
) -> Result<()> {
    let channel = open_channel(conn).await?;
    declare_mythic_exchange(&channel, MYTHIC_EXCHANGE).await?;
    declare_and_bind_queue(&channel, MYTHIC_EXCHANGE, routing_key).await?;

    info!("Resync listener ready on queue '{}'", routing_key);

    let mut consumer = channel
        .basic_consume(
            routing_key,
            &format!("{}_consumer", routing_key),
            BasicConsumeOptions { no_ack: false, ..Default::default() },
            FieldTable::default(),
        )
        .await
        .map_err(MythicError::from)?;

    while let Some(delivery_result) = futures_lite::StreamExt::next(&mut consumer).await {
        let delivery = match delivery_result {
            Ok(d) => d,
            Err(e) => return Err(MythicError::from(e)),
        };

        info!("Resync requested for '{}'", c2_name);

        // Re-send the sync message
        let sync_msg = crate::sync::build_sync_message(profile.clone(), parameters.to_vec());
        let response_bytes = match crate::sync::send_c2_sync(conn, sync_msg).await {
            Ok(()) => serde_json::to_vec(&C2RPCReSyncMessageResponse { success: true, error: String::new() })?,
            Err(e) => serde_json::to_vec(&C2RPCReSyncMessageResponse { success: false, error: e.to_string() })?,
        };

        if let Some(reply_to) = delivery.properties.reply_to() {
            let props = if let Some(cid) = delivery.properties.correlation_id() {
                BasicProperties::default().with_correlation_id(cid.clone())
            } else {
                BasicProperties::default()
            };
            let _ = channel
                .basic_publish("", reply_to.as_str(), BasicPublishOptions::default(), &response_bytes, props)
                .await;
        }

        let _ = delivery.ack(BasicAckOptions::default()).await;
    }

    Err(MythicError::RpcError(format!("Resync consumer for '{}' ended unexpectedly", routing_key)))
}

// ---------------------------------------------------------------------------
// Start/Stop server listeners
// Mirrors C2RPCStartServer / C2RPCStopServer in the Go library.
// The CONTAINER is responsible for launching/stopping the server binary.
// ---------------------------------------------------------------------------

#[derive(Debug, serde::Serialize)]
pub struct StartServerResponse {
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
    #[serde(rename = "message")]
    pub message: String,
    #[serde(rename = "server_running")]
    pub server_running: bool,
}

#[derive(Debug, serde::Serialize)]
struct StopServerResponse {
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
    #[serde(rename = "message")]
    pub message: String,
    #[serde(rename = "server_running")]
    pub server_running: bool,
}

/// Shared state: the running server child process handle.
/// `pub` so `lib.rs` can create it and pass it to both start and stop listeners.
pub type ServerProcess = Arc<Mutex<Option<tokio::process::Child>>>;

pub async fn run_start_server_listener(
    config: RabbitMQConfig,
    c2_name: String,
    binary_path: Option<String>,
    folder_path: Option<String>,
    server_process: ServerProcess,
) {
    let routing_key = get_routing_key(&c2_name, C2_RPC_START_SERVER_ROUTING_KEY);

    loop {
        let conn = connect_with_retry(&config).await;
        if let Err(e) = run_start_server_loop(
            &conn,
            &routing_key,
            &c2_name,
            &binary_path,
            &folder_path,
            Arc::clone(&server_process),
        )
        .await
        {
            error!("start_server listener error: {}. Reconnecting in {}s...", e, RETRY_CONNECT_DELAY_SECS);
            tokio::time::sleep(Duration::from_secs(RETRY_CONNECT_DELAY_SECS)).await;
        }
    }
}

async fn run_start_server_loop(
    conn: &Connection,
    routing_key: &str,
    c2_name: &str,
    binary_path: &Option<String>,
    folder_path: &Option<String>,
    server_process: ServerProcess,
) -> Result<()> {
    let channel = open_channel(conn).await?;
    declare_mythic_exchange(&channel, MYTHIC_EXCHANGE).await?;
    declare_and_bind_queue(&channel, MYTHIC_EXCHANGE, routing_key).await?;

    info!("start_server listener ready on queue '{}'", routing_key);

    let mut consumer = channel
        .basic_consume(
            routing_key,
            &format!("{}_consumer", routing_key),
            BasicConsumeOptions { no_ack: false, ..Default::default() },
            FieldTable::default(),
        )
        .await
        .map_err(MythicError::from)?;

    while let Some(delivery_result) = futures_lite::StreamExt::next(&mut consumer).await {
        let delivery = match delivery_result {
            Ok(d) => d,
            Err(e) => return Err(MythicError::from(e)),
        };

        info!("start_server RPC received for '{}'", c2_name);

        let response = launch_server(binary_path, folder_path, Arc::clone(&server_process)).await;
        let response_bytes = serde_json::to_vec(&response).unwrap_or_default();

        // Send status update to Mythic so the UI reflects the new state
        let _ = crate::sync::send_c2_update_status(conn, c2_name, response.server_running, &response.error).await;

        if let Some(reply_to) = delivery.properties.reply_to() {
            let props = if let Some(cid) = delivery.properties.correlation_id() {
                BasicProperties::default().with_correlation_id(cid.clone())
            } else {
                BasicProperties::default()
            };
            let _ = channel
                .basic_publish("", reply_to.as_str(), BasicPublishOptions::default(), &response_bytes, props)
                .await;
        }

        let _ = delivery.ack(BasicAckOptions::default()).await;
    }

    Err(MythicError::RpcError(format!("start_server consumer for '{}' ended unexpectedly", routing_key)))
}

pub async fn launch_server(
    binary_path: &Option<String>,
    folder_path: &Option<String>,
    server_process: ServerProcess,
) -> StartServerResponse {
    // Check if already running
    {
        let mut guard = server_process.lock().unwrap();
        if let Some(child) = guard.as_mut() {
            match child.try_wait() {
                Ok(None) => {
                    return StartServerResponse {
                        success: true,
                        error: String::new(),
                        message: "Server is already running".to_string(),
                        server_running: true,
                    };
                }
                _ => {
                    // Process has exited, clear it
                    *guard = None;
                }
            }
        }
    }

    let bin = match binary_path {
        Some(p) => p.clone(),
        None => {
            return StartServerResponse {
                success: false,
                error: "No server_binary_path configured".to_string(),
                message: String::new(),
                server_running: false,
            };
        }
    };

    // Resolve to absolute path, mirroring filepath.Abs() in Go
    let abs_bin = match std::fs::canonicalize(&bin) {
        Ok(p) => p,
        Err(e) => {
            return StartServerResponse {
                success: false,
                error: format!("Cannot resolve binary path '{}': {}", bin, e),
                message: String::new(),
                server_running: false,
            };
        }
    };

    // Working directory: use folder_path if set, else parent of binary
    let work_dir: PathBuf = folder_path
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| abs_bin.parent().unwrap_or(&abs_bin).to_path_buf());

    info!("Launching server binary: {:?} in {:?}", abs_bin, work_dir);

    match Command::new(&abs_bin)
        .current_dir(&work_dir)
        .envs(std::env::vars())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => {
            *server_process.lock().unwrap() = Some(child);
            // Give the process a moment to fail fast on startup errors
            tokio::time::sleep(Duration::from_secs(1)).await;

            let still_running = server_process
                .lock()
                .unwrap()
                .as_mut()
                .map(|c| matches!(c.try_wait(), Ok(None)))
                .unwrap_or(false);

            StartServerResponse {
                success: still_running,
                error: if still_running { String::new() } else { "Server process exited immediately".to_string() },
                message: format!("Server binary launched: {:?}", abs_bin),
                server_running: still_running,
            }
        }
        Err(e) => StartServerResponse {
            success: false,
            error: format!("Failed to launch server binary: {}", e),
            message: String::new(),
            server_running: false,
        },
    }
}

pub async fn run_stop_server_listener(config: RabbitMQConfig, c2_name: String, server_process: ServerProcess) {
    let routing_key = get_routing_key(&c2_name, C2_RPC_STOP_SERVER_ROUTING_KEY);

    loop {
        let conn = connect_with_retry(&config).await;
        if let Err(e) = run_stop_server_loop(&conn, &routing_key, &c2_name, Arc::clone(&server_process)).await {
            error!("stop_server listener error: {}. Reconnecting in {}s...", e, RETRY_CONNECT_DELAY_SECS);
            tokio::time::sleep(Duration::from_secs(RETRY_CONNECT_DELAY_SECS)).await;
        }
    }
}

async fn run_stop_server_loop(
    conn: &Connection,
    routing_key: &str,
    c2_name: &str,
    server_process: ServerProcess,
) -> Result<()> {
    let channel = open_channel(conn).await?;
    declare_mythic_exchange(&channel, MYTHIC_EXCHANGE).await?;
    declare_and_bind_queue(&channel, MYTHIC_EXCHANGE, routing_key).await?;

    info!("stop_server listener ready on queue '{}'", routing_key);

    let mut consumer = channel
        .basic_consume(
            routing_key,
            &format!("{}_consumer", routing_key),
            BasicConsumeOptions { no_ack: false, ..Default::default() },
            FieldTable::default(),
        )
        .await
        .map_err(MythicError::from)?;

    while let Some(delivery_result) = futures_lite::StreamExt::next(&mut consumer).await {
        let delivery = match delivery_result {
            Ok(d) => d,
            Err(e) => return Err(MythicError::from(e)),
        };

        info!("stop_server RPC received for '{}'", c2_name);

        // Kill the child process via the shared handle.
        // Take the child out of the mutex before awaiting kill() so we don't
        // hold the lock across an await point.
        let child_to_kill = {
            let mut guard = server_process.lock().unwrap();
            match guard.as_mut() {
                None => None, // already stopped
                Some(child) => match child.try_wait() {
                    Ok(Some(_)) => {
                        // exited naturally
                        *guard = None;
                        None
                    }
                    _ => guard.take(), // take ownership to kill outside the lock
                },
            }
        };

        let (success, error_msg) = if let Some(mut child) = child_to_kill {
            match child.kill().await {
                Ok(()) => {
                    info!("Server process killed for '{}'", c2_name);
                    (true, String::new())
                }
                Err(e) => {
                    error!("Failed to kill server process for '{}': {}", c2_name, e);
                    // Put it back since kill failed
                    *server_process.lock().unwrap() = Some(child);
                    (false, format!("Failed to kill process: {}", e))
                }
            }
        } else {
            (true, String::new()) // was already stopped
        };

        let response = StopServerResponse {
            success,
            error: error_msg.clone(),
            message: if success { "Server stopped".to_string() } else { String::new() },
            server_running: !success,
        };
        let response_bytes = serde_json::to_vec(&response).unwrap_or_default();

        let _ = crate::sync::send_c2_update_status(conn, c2_name, !success, &error_msg).await;

        if let Some(reply_to) = delivery.properties.reply_to() {
            let props = if let Some(cid) = delivery.properties.correlation_id() {
                BasicProperties::default().with_correlation_id(cid.clone())
            } else {
                BasicProperties::default()
            };
            let _ = channel
                .basic_publish("", reply_to.as_str(), BasicPublishOptions::default(), &response_bytes, props)
                .await;
        }

        let _ = delivery.ack(BasicAckOptions::default()).await;
    }

    Err(MythicError::RpcError(format!("stop_server consumer for '{}' ended unexpectedly", routing_key)))
}
