use tracing::debug;

pub const RABBITMQ_PORT: u16 = 5672;
pub const RABBITMQ_HOST: &str = "mythic_rabbitmq";
pub const RABBITMQ_VHOST: &str = "mythic_vhost";
pub const RABBITMQ_USER: &str = "mythic_user";

/// Helper struct to get the correct values to the Mythic environment
#[derive(Debug)]
pub struct Environment {
    pub rabbitmq_server: String,
    pub rabbitmq_port: u16,
    pub rabbitmq_password: String,
    pub rabbitmq_vhost: String,
    pub rabbitmq_user: String,
}

impl Environment {
    pub fn initialize() -> Self {
        // Load environment variables from .env file if it exists
        // This doesn't override existing env vars, just loads them for use
        let _ = dotenv::dotenv();

        let rabbitmq_server =
            std::env::var("RABBITMQ_HOST").unwrap_or_else(|_| RABBITMQ_HOST.to_string());
        let rabbitmq_port = match std::env::var("RABBITMQ_PORT") {
            Ok(port_str) => port_str.parse::<u16>().unwrap_or(RABBITMQ_PORT),
            Err(_) => RABBITMQ_PORT,
        };
        let rabbitmq_password = std::env::var("RABBITMQ_PASSWORD").unwrap();
        let rabbitmq_vhost =
            std::env::var("RABBITMQ_VHOST").unwrap_or_else(|_| RABBITMQ_VHOST.to_string());
        let rabbitmq_user =
            std::env::var("RABBITMQ_USER").unwrap_or_else(|_| RABBITMQ_USER.to_string());

        debug!("RABBITMQ_HOST set to {}", rabbitmq_server);
        debug!("RABBITMQ_PORT set to {}", rabbitmq_port);
        debug!("RABBITMQ_VHOST set to {}", rabbitmq_vhost);
        debug!("RABBITMQ_USER set to {}", rabbitmq_user);

        Self {
            rabbitmq_server,
            rabbitmq_port,
            rabbitmq_password,
            rabbitmq_vhost,
            rabbitmq_user,
        }
    }
}
