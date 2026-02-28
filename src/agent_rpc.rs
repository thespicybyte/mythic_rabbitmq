// PT (Payload Type) RPC receiver logic.
// Mirrors the individual recv_pt_*.go files from MythicMeta/MythicContainer.
//
// Key difference from C2 RPC (rpc.rs):
//   - C2 sends responses to `delivery.reply_to` (RPC pattern)
//   - PT sends responses to a FIXED response routing key on `mythic_exchange`
//     (fire-and-receive pattern, not reply_to)
//
// Each PT handler is spawned as a tokio task that:
//   1. Declares a durable queue bound to `mythic_exchange` with the namespaced
//      routing key (e.g. "apollo_pt_task_create_tasking").
//   2. Consumes messages with manual ack.
//   3. Calls the user-supplied handler function with the deserialized request.
//   4. Publishes the serialized response to the fixed response_routing_key.
//   5. Acks the delivery.
//   6. On any error, reconnects and retries.

use std::time::Duration;

use lapin::{
    BasicProperties, Connection,
    options::{BasicAckOptions, BasicConsumeOptions, BasicNackOptions, BasicPublishOptions},
    types::FieldTable,
};
use serde::{Serialize, de::DeserializeOwned};
use tracing::{debug, error, info, warn};

use crate::connection::{
    RabbitMQConfig, connect_with_retry, declare_and_bind_queue, declare_mythic_exchange,
    open_channel,
};
use crate::constants::{
    MYTHIC_EXCHANGE, PT_RPC_RESYNC_ROUTING_KEY, PT_SYNC_ROUTING_KEY, RETRY_CONNECT_DELAY_SECS,
    get_routing_key,
};
use crate::error::{MythicError, Result};
use crate::structs::{Command, PayloadType, PTRPCReSyncMessage, PTRPCReSyncMessageResponse};

/// Spawn a long-running tokio task that:
///   - Listens on `inbound_routing_key` for requests
///   - Calls `handler(request)` to produce a response
///   - Publishes the response to `response_routing_key` on `mythic_exchange`
///
/// This is the PT equivalent of `spawn_rpc_listener` in rpc.rs, except
/// responses go to a fixed key rather than `delivery.reply_to`.
pub fn spawn_pt_receiver<Req, Resp, F>(
    config: RabbitMQConfig,
    inbound_routing_key: String,
    response_routing_key: &'static str,
    handler: F,
) where
    Req: DeserializeOwned + Send + 'static,
    Resp: Serialize + Send + 'static,
    F: Fn(Req) -> Resp + Send + Sync + 'static,
{
    tokio::spawn(async move {
        loop {
            let conn = connect_with_retry(&config).await;
            if let Err(e) = run_pt_receiver_loop::<Req, Resp, F>(
                &conn,
                &inbound_routing_key,
                response_routing_key,
                &handler,
            )
            .await
            {
                error!(
                    "PT receiver for '{}' encountered an error: {}. Reconnecting in {}s...",
                    inbound_routing_key, e, RETRY_CONNECT_DELAY_SECS
                );
                tokio::time::sleep(Duration::from_secs(RETRY_CONNECT_DELAY_SECS)).await;
            }
        }
    });
}

async fn run_pt_receiver_loop<Req, Resp, F>(
    conn: &Connection,
    inbound_routing_key: &str,
    response_routing_key: &str,
    handler: &F,
) -> Result<()>
where
    Req: DeserializeOwned,
    Resp: Serialize,
    F: Fn(Req) -> Resp,
{
    let channel = open_channel(conn).await?;
    declare_mythic_exchange(&channel, MYTHIC_EXCHANGE).await?;
    declare_and_bind_queue(&channel, MYTHIC_EXCHANGE, inbound_routing_key).await?;

    info!("PT receiver ready on queue '{}'", inbound_routing_key);

    let mut consumer = channel
        .basic_consume(
            inbound_routing_key,
            &format!("{}_consumer", inbound_routing_key),
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
                error!(
                    "Error receiving delivery on '{}': {}",
                    inbound_routing_key, e
                );
                return Err(MythicError::from(e));
            }
        };

        debug!("Received PT message on '{}'", inbound_routing_key);

        // Deserialize request
        let request: Req = match serde_json::from_slice(&delivery.data) {
            Ok(r) => r,
            Err(e) => {
                warn!(
                    "Failed to deserialize request on '{}': {}",
                    inbound_routing_key, e
                );
                let _ = delivery
                    .nack(BasicNackOptions {
                        requeue: false,
                        ..Default::default()
                    })
                    .await;
                continue;
            }
        };

        // Call the user handler
        let response = handler(request);

        // Serialize response
        let response_bytes = match serde_json::to_vec(&response) {
            Ok(b) => b,
            Err(e) => {
                error!(
                    "Failed to serialize response on '{}': {}",
                    inbound_routing_key, e
                );
                let _ = delivery
                    .nack(BasicNackOptions {
                        requeue: false,
                        ..Default::default()
                    })
                    .await;
                continue;
            }
        };

        // Publish response to the fixed response routing key (NOT reply_to)
        if let Err(e) = channel
            .basic_publish(
                MYTHIC_EXCHANGE,
                response_routing_key,
                BasicPublishOptions::default(),
                &response_bytes,
                BasicProperties::default().with_content_type("application/json".into()),
            )
            .await
        {
            error!(
                "Failed to publish PT response on '{}': {}",
                response_routing_key, e
            );
            let _ = delivery
                .nack(BasicNackOptions {
                    requeue: true,
                    ..Default::default()
                })
                .await;
            continue;
        }

        // Ack after successful response send
        if let Err(e) = delivery.ack(BasicAckOptions::default()).await {
            error!(
                "Failed to ack delivery on '{}': {}",
                inbound_routing_key, e
            );
            return Err(MythicError::from(e));
        }
    }

    Err(MythicError::RpcError(format!(
        "PT consumer stream for '{}' ended unexpectedly",
        inbound_routing_key
    )))
}

// ---------------------------------------------------------------------------
// PT Resync listener
// ---------------------------------------------------------------------------

/// Listen on the PT resync queue. When Mythic asks us to resync, re-send the
/// full PT sync message. Mirrors the PT resync handler in the Go library.
pub async fn run_pt_resync_listener(
    config: RabbitMQConfig,
    pt_name: String,
    payload_type: PayloadType,
    commands: Vec<Command>,
) {
    let routing_key = get_routing_key(&pt_name, PT_RPC_RESYNC_ROUTING_KEY);

    loop {
        let conn = connect_with_retry(&config).await;
        if let Err(e) =
            run_pt_resync_loop(&conn, &routing_key, &pt_name, &payload_type, &commands).await
        {
            error!(
                "PT resync listener error: {}. Reconnecting in {}s...",
                e, RETRY_CONNECT_DELAY_SECS
            );
            tokio::time::sleep(Duration::from_secs(RETRY_CONNECT_DELAY_SECS)).await;
        }
    }
}

async fn run_pt_resync_loop(
    conn: &Connection,
    routing_key: &str,
    pt_name: &str,
    payload_type: &PayloadType,
    commands: &[Command],
) -> Result<()> {
    let channel = open_channel(conn).await?;
    declare_mythic_exchange(&channel, MYTHIC_EXCHANGE).await?;
    declare_and_bind_queue(&channel, MYTHIC_EXCHANGE, routing_key).await?;

    info!("PT resync listener ready on queue '{}'", routing_key);

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
            Err(e) => return Err(MythicError::from(e)),
        };

        // Parse the resync request (we don't use the content, but validate it)
        if let Err(e) = serde_json::from_slice::<PTRPCReSyncMessage>(&delivery.data) {
            warn!("Failed to parse PT resync message: {}", e);
        } else {
            info!("PT resync requested for '{}'", pt_name);
        }

        // Re-send the sync message
        let sync_msg =
            crate::agent_sync::build_pt_sync_message(payload_type.clone(), commands.to_vec());
        let (success, error) = match crate::agent_sync::send_pt_sync(conn, sync_msg).await {
            Ok(()) => (true, String::new()),
            Err(e) => (false, e.to_string()),
        };

        let response_bytes = serde_json::to_vec(&PTRPCReSyncMessageResponse { success, error })
            .unwrap_or_default();

        // PT resync response goes to the fixed sync routing key reply pattern
        // (reply_to if present, otherwise drop - Go library uses reply_to here)
        if let Some(reply_to) = delivery.properties.reply_to() {
            let props = if let Some(cid) = delivery.properties.correlation_id() {
                BasicProperties::default().with_correlation_id(cid.clone())
            } else {
                BasicProperties::default()
            };
            let _ = channel
                .basic_publish(
                    "",
                    reply_to.as_str(),
                    BasicPublishOptions::default(),
                    &response_bytes,
                    props,
                )
                .await;
        } else {
            // Fallback: publish to the PT sync routing key
            let _ = channel
                .basic_publish(
                    MYTHIC_EXCHANGE,
                    PT_SYNC_ROUTING_KEY,
                    BasicPublishOptions::default(),
                    &response_bytes,
                    BasicProperties::default(),
                )
                .await;
        }

        let _ = delivery.ack(BasicAckOptions::default()).await;
    }

    Err(MythicError::RpcError(format!(
        "PT resync consumer for '{}' ended unexpectedly",
        routing_key
    )))
}
