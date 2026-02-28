// PT (Payload Type) sync logic.
// Mirrors send_pt_sync_data.go from MythicMeta/MythicContainer.
//
// Flow:
//   1. Build a PayloadTypeSyncMessage from the payload type definition + commands.
//   2. Publish it to `mythic_exchange` with routing key `pt_sync`, using the
//      RabbitMQ direct-reply-to RPC pattern (amq.rabbitmq.reply-to queue).
//   3. Wait for the PayloadTypeSyncMessageResponse. Retry forever on failure.

use std::time::Duration;

use lapin::{
    BasicProperties, Connection,
    options::{BasicConsumeOptions, BasicPublishOptions},
    types::FieldTable,
};
use tracing::{error, info};
use uuid::Uuid;

use crate::constants::{
    CONTAINER_VERSION, MYTHIC_EXCHANGE, PT_SYNC_ROUTING_KEY, RETRY_CONNECT_DELAY_SECS,
    RPC_TIMEOUT_SECS,
};
use crate::connection::{declare_mythic_exchange, open_channel};
use crate::error::{MythicError, Result};
use crate::structs::{Command, PayloadType, PayloadTypeSyncMessage, PayloadTypeSyncMessageResponse};

/// Build a PT sync message, stamping in the container version.
pub fn build_pt_sync_message(
    payload_type: PayloadType,
    commands: Vec<Command>,
) -> PayloadTypeSyncMessage {
    PayloadTypeSyncMessage {
        payload_type,
        commands,
        container_version: CONTAINER_VERSION.to_string(),
    }
}

/// Send the PT sync message to Mythic and wait for a success response.
/// Retries indefinitely until Mythic acknowledges the registration.
pub async fn send_pt_sync(conn: &Connection, message: PayloadTypeSyncMessage) -> Result<()> {
    let body = serde_json::to_vec(&message)?;

    loop {
        match try_send_pt_sync(conn, &body, &message.payload_type.name).await {
            Ok(()) => {
                info!(
                    "Payload type '{}' registered successfully with Mythic",
                    message.payload_type.name
                );
                return Ok(());
            }
            Err(e) => {
                error!(
                    "PT sync failed: {}. Retrying in {}s...",
                    e, RETRY_CONNECT_DELAY_SECS
                );
                tokio::time::sleep(Duration::from_secs(RETRY_CONNECT_DELAY_SECS)).await;
            }
        }
    }
}

/// Single attempt at the RPC sync. Returns Err on any transport or protocol error.
async fn try_send_pt_sync(conn: &Connection, body: &[u8], pt_name: &str) -> Result<()> {
    let channel = open_channel(conn).await?;
    declare_mythic_exchange(&channel, MYTHIC_EXCHANGE).await?;

    let reply_queue = "amq.rabbitmq.reply-to";

    // Start consuming reply-to queue BEFORE publishing (avoids a race).
    let mut consumer = channel
        .basic_consume(
            reply_queue,
            &format!("{}_pt_sync_reply_consumer", pt_name),
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
            PT_SYNC_ROUTING_KEY,
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

    info!("PT sync message published, waiting for response...");

    let timeout = Duration::from_secs(RPC_TIMEOUT_SECS);
    let deadline = tokio::time::Instant::now() + timeout;

    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(MythicError::SyncFailed(
                "Timed out waiting for PT sync response".into(),
            ));
        }

        match tokio::time::timeout(remaining, futures_lite::StreamExt::next(&mut consumer)).await {
            Ok(Some(Ok(delivery))) => {
                if delivery
                    .properties
                    .correlation_id()
                    .as_ref()
                    .map(|s| s.as_str())
                    == Some(&correlation_id)
                {
                    let response: PayloadTypeSyncMessageResponse =
                        serde_json::from_slice(&delivery.data)?;
                    if response.success {
                        return Ok(());
                    } else {
                        return Err(MythicError::SyncFailed(response.error));
                    }
                }
                // Not our correlation id - ignore and keep waiting
            }
            Ok(Some(Err(e))) => return Err(MythicError::from(e)),
            Ok(None) => {
                return Err(MythicError::SyncFailed("Reply consumer closed".into()))
            }
            Err(_) => {
                return Err(MythicError::SyncFailed(
                    "Timed out waiting for PT sync response".into(),
                ))
            }
        }
    }
}
