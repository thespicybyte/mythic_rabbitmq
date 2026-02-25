// RabbitMQ connection management.
// Mirrors the connection handling in rabbitmq/utils.go from MythicMeta/MythicContainer.

use std::time::Duration;

use lapin::{
    options::{ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions},
    types::FieldTable,
    Channel, Connection, ConnectionProperties, ExchangeKind,
};
use tracing::{error, info};

use crate::error::{MythicError, Result};
use crate::{constants::RETRY_CONNECT_DELAY_SECS, environment::Environment};

/// Configuration read from environment variables, mirroring how the Go library
/// reads RABBITMQ_HOST and RABBITMQ_PASSWORD.
#[derive(Debug, Clone)]
pub struct RabbitMQConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub vhost: String,
}

impl RabbitMQConfig {
    /// Build config from environment variables, with sensible defaults.
    pub fn from_env() -> Self {
        let env = Environment::initialize();
        RabbitMQConfig {
            host: env.rabbitmq_server,
            port: env.rabbitmq_port,
            username: env.rabbitmq_user,
            password: env.rabbitmq_password,
            vhost: env.rabbitmq_vhost,
        }
    }

    pub fn amqp_url(&self) -> String {
        format!(
            "amqp://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.vhost,
        )
    }
}

/// Attempt to connect to RabbitMQ, retrying forever with a delay between attempts.
/// This mirrors the retry loop used throughout the Go MythicContainer library.
pub async fn connect_with_retry(config: &RabbitMQConfig) -> Connection {
    let url = config.amqp_url();
    loop {
        info!(
            "Connecting to RabbitMQ at {}:{}...",
            config.host, config.port
        );
        match Connection::connect(&url, ConnectionProperties::default()).await {
            Ok(conn) => {
                info!("Connected to RabbitMQ successfully");
                return conn;
            }
            Err(e) => {
                error!(
                    "Failed to connect to RabbitMQ: {}. Retrying in {}s...",
                    e, RETRY_CONNECT_DELAY_SECS
                );
                tokio::time::sleep(Duration::from_secs(RETRY_CONNECT_DELAY_SECS)).await;
            }
        }
    }
}

/// Open a fresh channel from the connection.
pub async fn open_channel(conn: &Connection) -> Result<Channel> {
    conn.create_channel().await.map_err(MythicError::from)
}

/// Declare the Mythic direct exchange.
/// Mirrors ExchangeDeclare in rabbitmq/utils.go:
///   durable=true, auto_delete=true, internal=false, no_wait=false
pub async fn declare_mythic_exchange(channel: &Channel, exchange: &str) -> Result<()> {
    channel
        .exchange_declare(
            exchange,
            ExchangeKind::Direct,
            ExchangeDeclareOptions {
                durable: true,
                auto_delete: true,
                internal: false,
                nowait: false,
                passive: false,
            },
            FieldTable::default(),
        )
        .await
        .map_err(MythicError::from)
}

/// Declare a durable queue and bind it to the exchange with the given routing key.
/// Mirrors the queue setup in ReceiveFromRPCQueue.
pub async fn declare_and_bind_queue(
    channel: &Channel,
    exchange: &str,
    routing_key: &str,
) -> Result<()> {
    channel
        .queue_declare(
            routing_key,
            QueueDeclareOptions {
                durable: false,
                auto_delete: true,
                exclusive: false,
                nowait: false,
                passive: false,
            },
            FieldTable::default(),
        )
        .await
        .map_err(MythicError::from)?;

    channel
        .queue_bind(
            routing_key,
            exchange,
            routing_key,
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await
        .map_err(MythicError::from)?;

    Ok(())
}
