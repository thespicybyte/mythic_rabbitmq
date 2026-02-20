//! # mythic-rabbitmq
//!
//! Rust port of the RabbitMQ layer from [MythicMeta/MythicContainer](https://github.com/MythicMeta/MythicContainer).
//!
//! Handles C2 profile registration and RPC dispatch for Mythic C2 containers.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use mythic_rabbitmq::{MythicC2Container, C2ProfileDefinition, C2Parameter};
//!
//! #[tokio::main]
//! async fn main() {
//!     let container = MythicC2Container::builder()
//!         .profile(C2ProfileDefinition {
//!             name: "reverse_tcp".to_string(),
//!             description: "...".to_string(),
//!             author: "@spicybyte".to_string(),
//!             is_p2p: false,
//!             is_server_routed: true,
//!             mythic_encrypts: false,
//!             server_binary_path: "./reverse_tcp/c2_code/reverse_tcp_server".to_string(),
//!             server_folder_path: "./reverse_tcp/c2_code".to_string(),
//!         })
//!         .parameters(vec![/* ... */])
//!         .on_config_check(|msg| {
//!             mythic_rabbitmq::C2ConfigCheckMessageResponse {
//!                 success: true,
//!                 error: String::new(),
//!                 message: "ok".to_string(),
//!                 restart_internal_server: false,
//!             }
//!         })
//!         .build();
//!
//!     container.start_and_run_forever().await;
//! }
//! ```

pub mod constants;
pub mod connection;
pub mod error;
pub mod rpc;
pub mod structs;
pub mod sync;

// Re-export the most-used types at crate root for ergonomics.
pub use connection::RabbitMQConfig;
pub use error::{MythicError, Result};
pub use structs::{
    C2ConfigCheckMessage, C2ConfigCheckMessageResponse,
    C2GetIOCMessage, C2GetIOCMessageResponse,
    C2GetRedirectorRuleMessage, C2GetRedirectorRuleMessageResponse,
    C2HostFileMessage, C2HostFileMessageResponse,
    C2OPSECMessage, C2OPSECMessageResponse,
    C2Parameter, C2ParameterDictionary, C2ProfileDefinition,
    C2SampleMessageMessage, C2SampleMessageResponse,
    C2SyncMessage, C2SyncMessageResponse, IOC,
};

use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{error, info};

use constants::{
    C2_RPC_CONFIG_CHECK_ROUTING_KEY, C2_RPC_GET_IOC_ROUTING_KEY,
    C2_RPC_HOST_FILE_ROUTING_KEY, C2_RPC_OPSEC_CHECK_ROUTING_KEY,
    C2_RPC_REDIRECTOR_RULES_ROUTING_KEY,
    C2_RPC_SAMPLE_MESSAGE_ROUTING_KEY, RETRY_CONNECT_DELAY_SECS, get_routing_key,
};
pub use structs::MythicRPCC2UpdateStatusMessage;

// ---------------------------------------------------------------------------
// Handler type aliases
// ---------------------------------------------------------------------------

pub type ConfigCheckHandler = fn(C2ConfigCheckMessage) -> C2ConfigCheckMessageResponse;
pub type OpsecCheckHandler = fn(C2OPSECMessage) -> C2OPSECMessageResponse;
pub type GetIocHandler = fn(C2GetIOCMessage) -> C2GetIOCMessageResponse;
pub type SampleMessageHandler = fn(C2SampleMessageMessage) -> C2SampleMessageResponse;
pub type RedirectorRulesHandler = fn(C2GetRedirectorRuleMessage) -> C2GetRedirectorRuleMessageResponse;
pub type HostFileHandler = fn(C2HostFileMessage) -> C2HostFileMessageResponse;

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Builder for [`MythicC2Container`].
#[derive(Default)]
pub struct MythicC2ContainerBuilder {
    profile: Option<C2ProfileDefinition>,
    parameters: Vec<C2Parameter>,
    config: Option<RabbitMQConfig>,
    /// Path to the server binary the container will launch on start_server RPC.
    /// Mirrors ServerBinaryPath (json:"-") from the Go C2Profile struct.
    server_binary_path: Option<String>,
    /// Working directory for the server process.
    /// Mirrors ServerFolderPath (json:"-") from the Go C2Profile struct.
    server_folder_path: Option<String>,
    on_config_check: Option<ConfigCheckHandler>,
    on_opsec_check: Option<OpsecCheckHandler>,
    on_get_ioc: Option<GetIocHandler>,
    on_sample_message: Option<SampleMessageHandler>,
    on_redirector_rules: Option<RedirectorRulesHandler>,
    on_host_file: Option<HostFileHandler>,
}

impl MythicC2ContainerBuilder {
    pub fn profile(mut self, p: C2ProfileDefinition) -> Self {
        self.profile = Some(p);
        self
    }

    pub fn parameters(mut self, params: Vec<C2Parameter>) -> Self {
        self.parameters = params;
        self
    }

    /// Override the RabbitMQ config. Defaults to [`RabbitMQConfig::from_env`].
    pub fn rabbitmq_config(mut self, config: RabbitMQConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Path to the server binary. The container launches this process when
    /// Mythic sends a start_server RPC. Mirrors Go's ServerBinaryPath (json:"-").
    pub fn server_binary_path(mut self, path: impl Into<String>) -> Self {
        self.server_binary_path = Some(path.into());
        self
    }

    /// Working directory for the server process (defaults to the binary's parent dir).
    pub fn server_folder_path(mut self, path: impl Into<String>) -> Self {
        self.server_folder_path = Some(path.into());
        self
    }

    pub fn on_config_check(mut self, f: ConfigCheckHandler) -> Self {
        self.on_config_check = Some(f);
        self
    }

    pub fn on_opsec_check(mut self, f: OpsecCheckHandler) -> Self {
        self.on_opsec_check = Some(f);
        self
    }

    pub fn on_get_ioc(mut self, f: GetIocHandler) -> Self {
        self.on_get_ioc = Some(f);
        self
    }

    pub fn on_sample_message(mut self, f: SampleMessageHandler) -> Self {
        self.on_sample_message = Some(f);
        self
    }

    pub fn on_redirector_rules(mut self, f: RedirectorRulesHandler) -> Self {
        self.on_redirector_rules = Some(f);
        self
    }

    pub fn on_host_file(mut self, f: HostFileHandler) -> Self {
        self.on_host_file = Some(f);
        self
    }

    pub fn build(self) -> MythicC2Container {
        MythicC2Container {
            profile: self.profile.expect("C2ProfileDefinition is required"),
            parameters: self.parameters,
            config: self.config.unwrap_or_else(RabbitMQConfig::from_env),
            server_binary_path: self.server_binary_path,
            server_folder_path: self.server_folder_path,
            on_config_check: self.on_config_check,
            on_opsec_check: self.on_opsec_check,
            on_get_ioc: self.on_get_ioc,
            on_sample_message: self.on_sample_message,
            on_redirector_rules: self.on_redirector_rules,
            on_host_file: self.on_host_file,
        }
    }
}

// ---------------------------------------------------------------------------
// Main container type
// ---------------------------------------------------------------------------

/// Mirrors `MythicContainer.StartAndRunForever` from the Go library, scoped to
/// C2 profile services.
pub struct MythicC2Container {
    pub profile: C2ProfileDefinition,
    pub parameters: Vec<C2Parameter>,
    pub config: RabbitMQConfig,
    /// Local-only binary path - NOT sent to Mythic (mirrors json:"-" in Go).
    pub server_binary_path: Option<String>,
    /// Local-only working directory - NOT sent to Mythic (mirrors json:"-" in Go).
    pub server_folder_path: Option<String>,
    // Optional handlers
    pub on_config_check: Option<ConfigCheckHandler>,
    pub on_opsec_check: Option<OpsecCheckHandler>,
    pub on_get_ioc: Option<GetIocHandler>,
    pub on_sample_message: Option<SampleMessageHandler>,
    pub on_redirector_rules: Option<RedirectorRulesHandler>,
    pub on_host_file: Option<HostFileHandler>,
}

impl MythicC2Container {
    pub fn builder() -> MythicC2ContainerBuilder {
        MythicC2ContainerBuilder::default()
    }

    /// Connect to RabbitMQ, sync the profile definition, then spawn all RPC
    /// listener tasks and block forever.
    ///
    /// Mirrors `StartAndRunForever` + `Initialize` from the Go library.
    pub async fn start_and_run_forever(self) {
        let c2_name = self.profile.name.clone();
        let config = self.config.clone();

        // Shared handle to the running server process - used by both the
        // auto-launch on startup and the start/stop RPC listeners.
        let server_process: rpc::ServerProcess = Arc::new(Mutex::new(None));

        // 1. Connect and sync - retry until Mythic acknowledges.
        loop {
            let conn = connection::connect_with_retry(&config).await;

            let sync_msg = sync::build_sync_message(
                self.profile.clone(),
                self.parameters.clone(),
            );

            match sync::send_c2_sync(&conn, sync_msg).await {
                Ok(()) => {
                    info!("C2 profile '{}' synced. Auto-launching server binary...", c2_name);

                    // 2. Auto-launch the server binary immediately on startup.
                    let launched = rpc::launch_server(
                        &self.server_binary_path,
                        &self.server_folder_path,
                        Arc::clone(&server_process),
                    ).await;

                    if launched.server_running {
                        info!("Server binary launched successfully for '{}'", c2_name);
                    } else {
                        error!("Server binary failed to launch for '{}': {}", c2_name, launched.error);
                    }

                    // 3. Tell Mythic the container is online, reflecting actual server state.
                    if let Err(e) = sync::send_c2_update_status(
                        &conn, &c2_name, launched.server_running, &launched.error,
                    ).await {
                        error!("Failed to send initial status update: {}", e);
                        tokio::time::sleep(Duration::from_secs(RETRY_CONNECT_DELAY_SECS)).await;
                        continue;
                    }

                    info!("C2 profile '{}' online. Spawning RPC listeners...", c2_name);

                    // 4. Spawn all RPC listener tasks, sharing the process handle.
                    self.spawn_rpc_listeners(&c2_name, config.clone(), Arc::clone(&server_process));

                    // 5. Keep the main task alive; listeners run in background tasks.
                    tokio::signal::ctrl_c()
                        .await
                        .expect("Failed to listen for Ctrl+C");

                    info!("Shutting down C2 container for '{}'", c2_name);
                    return;
                }
                Err(e) => {
                    error!("Sync failed: {}. Reconnecting...", e);
                    tokio::time::sleep(Duration::from_secs(RETRY_CONNECT_DELAY_SECS)).await;
                }
            }
        }
    }

    fn spawn_rpc_listeners(&self, c2_name: &str, config: RabbitMQConfig, server_process: rpc::ServerProcess) {
        let config_check = self.on_config_check.unwrap_or(default_config_check);
        let opsec_check = self.on_opsec_check.unwrap_or(default_opsec_check);
        let get_ioc = self.on_get_ioc.unwrap_or(default_get_ioc);
        let sample_message = self.on_sample_message.unwrap_or(default_sample_message);
        let redirector_rules = self.on_redirector_rules.unwrap_or(default_redirector_rules);
        let host_file = self.on_host_file.unwrap_or(default_host_file);

        rpc::spawn_rpc_listener(
            config.clone(),
            get_routing_key(c2_name, C2_RPC_CONFIG_CHECK_ROUTING_KEY),
            config_check,
        );

        rpc::spawn_rpc_listener(
            config.clone(),
            get_routing_key(c2_name, C2_RPC_OPSEC_CHECK_ROUTING_KEY),
            opsec_check,
        );

        rpc::spawn_rpc_listener(
            config.clone(),
            get_routing_key(c2_name, C2_RPC_GET_IOC_ROUTING_KEY),
            get_ioc,
        );

        rpc::spawn_rpc_listener(
            config.clone(),
            get_routing_key(c2_name, C2_RPC_SAMPLE_MESSAGE_ROUTING_KEY),
            sample_message,
        );

        rpc::spawn_rpc_listener(
            config.clone(),
            get_routing_key(c2_name, C2_RPC_REDIRECTOR_RULES_ROUTING_KEY),
            redirector_rules,
        );

        rpc::spawn_rpc_listener(
            config.clone(),
            get_routing_key(c2_name, C2_RPC_HOST_FILE_ROUTING_KEY),
            host_file,
        );

        // start_server and stop_server share the process handle so stop can
        // kill the exact child process that was launched.
        spawn_start_server_listener(
            config.clone(),
            c2_name.to_string(),
            self.server_binary_path.clone(),
            self.server_folder_path.clone(),
            Arc::clone(&server_process),
        );
        spawn_stop_server_listener(config.clone(), c2_name.to_string(), Arc::clone(&server_process));

        // resync: Mythic asks us to re-send the sync message
        let resync_profile = self.profile.clone();
        let resync_parameters = self.parameters.clone();
        let resync_config = config.clone();
        let resync_name = c2_name.to_string();
        tokio::spawn(async move {
            rpc::run_resync_listener(resync_config, resync_name, resync_profile, resync_parameters).await;
        });
    }
}

// ---------------------------------------------------------------------------
// Start/Stop server listeners
// ---------------------------------------------------------------------------

fn spawn_start_server_listener(
    config: RabbitMQConfig,
    c2_name: String,
    binary_path: Option<String>,
    folder_path: Option<String>,
    server_process: rpc::ServerProcess,
) {
    tokio::spawn(async move {
        rpc::run_start_server_listener(config, c2_name, binary_path, folder_path, server_process).await;
    });
}

fn spawn_stop_server_listener(config: RabbitMQConfig, c2_name: String, server_process: rpc::ServerProcess) {
    tokio::spawn(async move {
        rpc::run_stop_server_listener(config, c2_name, server_process).await;
    });
}

// ---------------------------------------------------------------------------
// Default (stub) handlers - used when the caller doesn't register one
// ---------------------------------------------------------------------------

fn default_config_check(_msg: C2ConfigCheckMessage) -> C2ConfigCheckMessageResponse {
    C2ConfigCheckMessageResponse {
        success: true,
        error: String::new(),
        message: "No config check handler registered".to_string(),
        restart_internal_server: false,
    }
}

fn default_opsec_check(_msg: C2OPSECMessage) -> C2OPSECMessageResponse {
    C2OPSECMessageResponse {
        success: true,
        error: String::new(),
        message: "No OPSEC check handler registered".to_string(),
        restart_internal_server: false,
    }
}

fn default_get_ioc(_msg: C2GetIOCMessage) -> C2GetIOCMessageResponse {
    C2GetIOCMessageResponse {
        success: true,
        error: String::new(),
        iocs: vec![],
        restart_internal_server: false,
    }
}

fn default_sample_message(_msg: C2SampleMessageMessage) -> C2SampleMessageResponse {
    C2SampleMessageResponse {
        success: false,
        error: String::new(),
        message: "Not supported".to_string(),
        restart_internal_server: false,
    }
}

fn default_redirector_rules(_msg: C2GetRedirectorRuleMessage) -> C2GetRedirectorRuleMessageResponse {
    C2GetRedirectorRuleMessageResponse {
        success: false,
        error: String::new(),
        message: "Not supported".to_string(),
        restart_internal_server: false,
    }
}

fn default_host_file(_msg: C2HostFileMessage) -> C2HostFileMessageResponse {
    C2HostFileMessageResponse {
        success: false,
        error: "Not supported".to_string(),
        restart_internal_server: false,
    }
}
