// C2 profile sync logic.
// Mirrors send_c2_sync_data.go from MythicMeta/MythicContainer.
//
// Flow:
//   1. Build a C2SyncMessage from the profile definition + parameters.
//   2. Publish it to `mythic_exchange` with routing key `c2_sync`, using the
//      RabbitMQ direct-reply-to RPC pattern (amq.rabbitmq.reply-to queue).
//   3. Wait for the C2SyncMessageResponse. Retry forever on failure.

use std::time::Duration;

use lapin::{
    BasicProperties, Connection,
    options::{BasicConsumeOptions, BasicPublishOptions},
    types::FieldTable,
};
use tracing::{error, info};
use uuid::Uuid;

use crate::constants::{C2_SYNC_ROUTING_KEY, CONTAINER_VERSION, MYTHIC_EXCHANGE, MYTHIC_RPC_C2_UPDATE_STATUS, RETRY_CONNECT_DELAY_SECS, RPC_TIMEOUT_SECS};
use crate::error::{MythicError, Result};
use crate::structs::{C2SyncMessage, C2SyncMessageResponse, MythicRPCC2UpdateStatusMessage};
use crate::connection::{declare_mythic_exchange, open_channel};

/// Send the C2 profile sync message to Mythic and wait for a success response.
/// Retries indefinitely until Mythic acknowledges the registration.
pub async fn send_c2_sync(conn: &Connection, message: C2SyncMessage) -> Result<()> {
    let body = serde_json::to_vec(&message)?;

    loop {
        match try_send_sync(conn, &body).await {
            Ok(()) => {
                info!("C2 profile '{}' registered successfully with Mythic", message.profile.name);
                return Ok(());
            }
            Err(e) => {
                error!("C2 sync failed: {}. Retrying in {}s...", e, RETRY_CONNECT_DELAY_SECS);
                tokio::time::sleep(Duration::from_secs(RETRY_CONNECT_DELAY_SECS)).await;
            }
        }
    }
}

/// Single attempt at the RPC sync. Returns Err on any transport or protocol error.
async fn try_send_sync(conn: &Connection, body: &[u8]) -> Result<()> {
    let channel = open_channel(conn).await?;
    declare_mythic_exchange(&channel, MYTHIC_EXCHANGE).await?;

    // Use the built-in RabbitMQ direct reply-to pseudo-queue for RPC.
    let reply_queue = "amq.rabbitmq.reply-to";

    // Start consuming reply-to queue BEFORE publishing (avoids a race).
    let mut consumer = channel
        .basic_consume(
            reply_queue,
            "c2_sync_reply_consumer",
            BasicConsumeOptions {
                no_ack: true, // direct reply-to requires no_ack=true
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .map_err(MythicError::from)?;

    let correlation_id = Uuid::new_v4().to_string();

    channel
        .basic_publish(
            MYTHIC_EXCHANGE,
            C2_SYNC_ROUTING_KEY,
            BasicPublishOptions::default(),
            body,
            BasicProperties::default()
                .with_correlation_id(correlation_id.clone().into())
                .with_reply_to(reply_queue.into())
                .with_content_type("application/json".into()),
        )
        .await
        .map_err(MythicError::from)?
        .await
        .map_err(MythicError::from)?;

    info!("C2 sync message published, waiting for response...");

    // Wait for the reply with a timeout.
    let timeout = Duration::from_secs(RPC_TIMEOUT_SECS);
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(MythicError::SyncFailed("Timed out waiting for sync response".into()));
        }

        match tokio::time::timeout(remaining, futures_lite::StreamExt::next(&mut consumer)).await {
            Ok(Some(Ok(delivery))) => {
                if delivery.properties.correlation_id().as_ref().map(|s| s.as_str()) == Some(&correlation_id) {
                    let response: C2SyncMessageResponse = serde_json::from_slice(&delivery.data)?;
                    if response.success {
                        return Ok(());
                    } else {
                        return Err(MythicError::SyncFailed(response.error));
                    }
                }
                // Not our correlation id - ignore and keep waiting
            }
            Ok(Some(Err(e))) => return Err(MythicError::from(e)),
            Ok(None) => return Err(MythicError::SyncFailed("Reply consumer closed".into())),
            Err(_) => return Err(MythicError::SyncFailed("Timed out waiting for sync response".into())),
        }
    }
}

/// Build a C2SyncMessage, stamping in the container version.
pub fn build_sync_message(
    profile: crate::structs::C2ProfileDefinition,
    parameters: Vec<crate::structs::C2Parameter>,
) -> C2SyncMessage {
    C2SyncMessage {
        profile,
        parameters,
        container_version: CONTAINER_VERSION.to_string(),
    }
}

/// Send `mythic_rpc_c2_update_status` to Mythic.
///
/// Mythic uses this to mark the container as online/offline in the UI.
/// Must be called once after a successful sync, and again whenever the
/// internal server starts or stops.
///
/// Mirrors SendMythicRPCC2UpdateStatus in send_c2_rpc_update_status.go.
pub async fn send_c2_update_status(
    conn: &Connection,
    c2_profile: &str,
    server_running: bool,
    error: &str,
) -> Result<()> {
    let msg = MythicRPCC2UpdateStatusMessage {
        c2_profile: c2_profile.to_string(),
        server_running,
        error: error.to_string(),
    };
    let body = serde_json::to_vec(&msg)?;

    loop {
        match try_send_rpc(conn, MYTHIC_RPC_C2_UPDATE_STATUS, &body).await {
            Ok(()) => {
                info!(
                    "C2 status update sent: profile={} server_running={}",
                    c2_profile, server_running
                );
                return Ok(());
            }
            Err(e) => {
                error!("Failed to send C2 status update: {}. Retrying in {}s...", e, RETRY_CONNECT_DELAY_SECS);
                tokio::time::sleep(Duration::from_secs(RETRY_CONNECT_DELAY_SECS)).await;
            }
        }
    }
}

/// Generic fire-and-forget RPC send (publish + await confirm, no reply needed).
async fn try_send_rpc(conn: &Connection, routing_key: &str, body: &[u8]) -> Result<()> {
    let channel = open_channel(conn).await?;
    declare_mythic_exchange(&channel, MYTHIC_EXCHANGE).await?;

    let reply_queue = "amq.rabbitmq.reply-to";

    let mut consumer = channel
        .basic_consume(
            reply_queue,
            &format!("{}_reply_consumer", routing_key),
            BasicConsumeOptions {
                no_ack: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
        .map_err(MythicError::from)?;

    let correlation_id = Uuid::new_v4().to_string();

    channel
        .basic_publish(
            MYTHIC_EXCHANGE,
            routing_key,
            BasicPublishOptions::default(),
            body,
            BasicProperties::default()
                .with_correlation_id(correlation_id.clone().into())
                .with_reply_to(reply_queue.into())
                .with_content_type("application/json".into()),
        )
        .await
        .map_err(MythicError::from)?
        .await
        .map_err(MythicError::from)?;

    // Wait for ack with timeout - we don't parse the response body, just confirm delivery.
    let timeout = Duration::from_secs(RPC_TIMEOUT_SECS);
    match tokio::time::timeout(timeout, futures_lite::StreamExt::next(&mut consumer)).await {
        Ok(Some(Ok(_))) => Ok(()),
        Ok(Some(Err(e))) => Err(MythicError::from(e)),
        Ok(None) => Err(MythicError::SyncFailed("Reply consumer closed".into())),
        Err(_) => Err(MythicError::SyncFailed(format!("Timed out waiting for {} response", routing_key))),
    }
}
