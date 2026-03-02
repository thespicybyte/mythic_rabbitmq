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
    CONTAINER_VERSION, MYTHIC_EXCHANGE, MYTHIC_RPC_PAYLOAD_UPDATE_BUILD_STEP, PT_SYNC_ROUTING_KEY,
    RETRY_CONNECT_DELAY_SECS, RPC_TIMEOUT_SECS,
};
use crate::connection::{connect_with_retry, declare_mythic_exchange, global_config, open_channel};
use crate::error::{MythicError, Result};
use crate::structs::{
    Command, MythicRPCPayloadUpdateBuildStepMessage, MythicRPCPayloadUpdateBuildStepMessageResponse,
    PayloadType, PayloadTypeSyncMessage, PayloadTypeSyncMessageResponse,
};

/// Send `mythic_rpc_payload_update_build_step` to Mythic during a payload build.
///
/// Call this from within your async build handler to update the progress
/// indicator in the Mythic UI for each declared [`BuildStep`].
/// No connection management is required — the container's global connection
/// is used automatically, mirroring Go's `SendMythicRPCPayloadUpdateBuildStep`.
///
/// # Example
/// ```rust,no_run
/// fn build(msg: PayloadBuildMessage) -> BoxFuture<PayloadBuildResponse> {
///     Box::pin(async move {
///         send_payload_update_build_step(MythicRPCPayloadUpdateBuildStepMessage {
///             payload_uuid: msg.uuid.clone(),
///             step_name: "compile".to_string(),
///             step_stdout: "Compiling...".to_string(),
///             step_success: true,
///             ..Default::default()
///         }).await.ok();
///
///         PayloadBuildResponse { uuid: msg.uuid, success: true, .. }
///     })
/// }
/// ```
pub async fn send_payload_update_build_step(
    msg: MythicRPCPayloadUpdateBuildStepMessage,
) -> Result<MythicRPCPayloadUpdateBuildStepMessageResponse> {
    let config = global_config()
        .ok_or_else(|| MythicError::RpcError("Global RabbitMQ config not initialized — is the container running?".into()))?;
    let conn = connect_with_retry(config).await;
    let body = serde_json::to_vec(&msg)?;
    try_send_pt_rpc(&conn, MYTHIC_RPC_PAYLOAD_UPDATE_BUILD_STEP, &body).await
}

/// Generic fire-and-wait RPC send for PT → Mythic calls.
async fn try_send_pt_rpc(
    conn: &Connection,
    routing_key: &str,
    body: &[u8],
) -> Result<MythicRPCPayloadUpdateBuildStepMessageResponse> {
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

    let timeout = Duration::from_secs(RPC_TIMEOUT_SECS);
    match tokio::time::timeout(timeout, futures_lite::StreamExt::next(&mut consumer)).await {
        Ok(Some(Ok(delivery))) => {
            let resp: MythicRPCPayloadUpdateBuildStepMessageResponse =
                serde_json::from_slice(&delivery.data)?;
            Ok(resp)
        }
        Ok(Some(Err(e))) => Err(MythicError::from(e)),
        Ok(None) => Err(MythicError::SyncFailed("Reply consumer closed".into())),
        Err(_) => Err(MythicError::SyncFailed(format!(
            "Timed out waiting for {} response",
            routing_key
        ))),
    }
}

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
