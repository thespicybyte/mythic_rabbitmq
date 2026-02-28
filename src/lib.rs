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

pub mod agent_rpc;
pub mod agent_sync;
pub mod connection;
pub mod constants;
mod environment;
pub mod error;
pub mod rpc;
pub mod structs;
pub mod sync;

// Re-export the most-used types at crate root for ergonomics.
pub use connection::RabbitMQConfig;
pub use error::{MythicError, Result};
pub use structs::{
    C2ConfigCheckMessage, C2ConfigCheckMessageResponse, C2GetIOCMessage, C2GetIOCMessageResponse,
    C2GetRedirectorRuleMessage, C2GetRedirectorRuleMessageResponse, C2HostFileMessage,
    C2HostFileMessageResponse, C2OPSECMessage, C2OPSECMessageResponse, C2Parameter,
    C2ParameterDictionary, C2ProfileDefinition, C2SampleMessageMessage, C2SampleMessageResponse,
    C2SyncMessage, C2SyncMessageResponse, IOC,
};

use environment::Environment;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{error, info};

use constants::{
    get_routing_key, C2_RPC_CONFIG_CHECK_ROUTING_KEY, C2_RPC_GET_IOC_ROUTING_KEY,
    C2_RPC_HOST_FILE_ROUTING_KEY, C2_RPC_OPSEC_CHECK_ROUTING_KEY,
    C2_RPC_REDIRECTOR_RULES_ROUTING_KEY, C2_RPC_SAMPLE_MESSAGE_ROUTING_KEY,
    RETRY_CONNECT_DELAY_SECS,
};
pub use structs::MythicRPCC2UpdateStatusMessage;

// Agent (payload type) re-exports
pub use structs::{
    BuildParameter, BuildStep, Command, CommandParameter, PayloadBuildMessage, PayloadBuildResponse,
    PayloadType, PayloadTypeSyncMessage,
};

// ---------------------------------------------------------------------------
// Handler type aliases
// ---------------------------------------------------------------------------

pub type ConfigCheckHandler = fn(C2ConfigCheckMessage) -> C2ConfigCheckMessageResponse;
pub type OpsecCheckHandler = fn(C2OPSECMessage) -> C2OPSECMessageResponse;
pub type GetIocHandler = fn(C2GetIOCMessage) -> C2GetIOCMessageResponse;
pub type SampleMessageHandler = fn(C2SampleMessageMessage) -> C2SampleMessageResponse;
pub type RedirectorRulesHandler =
    fn(C2GetRedirectorRuleMessage) -> C2GetRedirectorRuleMessageResponse;
pub type HostFileHandler = fn(C2HostFileMessage) -> C2HostFileMessageResponse;

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Builder for [`MythicC2Container`].
pub struct MythicC2ContainerBuilder {
    profile: Option<C2ProfileDefinition>,
    parameters: Vec<C2Parameter>,
    config: Option<RabbitMQConfig>,
    server_binary_path: Option<String>,
    server_folder_path: Option<String>,
    on_config_check: Option<ConfigCheckHandler>,
    on_opsec_check: Option<OpsecCheckHandler>,
    on_get_ioc: Option<GetIocHandler>,
    on_sample_message: Option<SampleMessageHandler>,
    on_redirector_rules: Option<RedirectorRulesHandler>,
    on_host_file: Option<HostFileHandler>,
}

impl Default for MythicC2ContainerBuilder {
    fn default() -> Self {
        Environment::initialize();
        Self {
            profile: Default::default(),
            parameters: Default::default(),
            config: Default::default(),
            server_binary_path: Default::default(),
            server_folder_path: Default::default(),
            on_config_check: Default::default(),
            on_opsec_check: Default::default(),
            on_get_ioc: Default::default(),
            on_sample_message: Default::default(),
            on_redirector_rules: Default::default(),
            on_host_file: Default::default(),
        }
    }
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

            let sync_msg = sync::build_sync_message(self.profile.clone(), self.parameters.clone());

            match sync::send_c2_sync(&conn, sync_msg).await {
                Ok(()) => {
                    info!(
                        "C2 profile '{}' synced. Auto-launching server binary...",
                        c2_name
                    );

                    // 2. Auto-launch the server binary immediately on startup.
                    let launched = rpc::launch_server(
                        &self.server_binary_path,
                        &self.server_folder_path,
                        Arc::clone(&server_process),
                    )
                    .await;

                    if launched.server_running {
                        info!("Server binary launched successfully for '{}'", c2_name);
                    } else {
                        error!(
                            "Server binary failed to launch for '{}': {}",
                            c2_name, launched.error
                        );
                    }

                    // 3. Tell Mythic the container is online, reflecting actual server state.
                    if let Err(e) = sync::send_c2_update_status(
                        &conn,
                        &c2_name,
                        launched.server_running,
                        &launched.error,
                    )
                    .await
                    {
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

    fn spawn_rpc_listeners(
        &self,
        c2_name: &str,
        config: RabbitMQConfig,
        server_process: rpc::ServerProcess,
    ) {
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
        spawn_stop_server_listener(
            config.clone(),
            c2_name.to_string(),
            Arc::clone(&server_process),
        );

        // resync: Mythic asks us to re-send the sync message
        let resync_profile = self.profile.clone();
        let resync_parameters = self.parameters.clone();
        let resync_config = config.clone();
        let resync_name = c2_name.to_string();
        tokio::spawn(async move {
            rpc::run_resync_listener(
                resync_config,
                resync_name,
                resync_profile,
                resync_parameters,
            )
            .await;
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
        rpc::run_start_server_listener(config, c2_name, binary_path, folder_path, server_process)
            .await;
    });
}

fn spawn_stop_server_listener(
    config: RabbitMQConfig,
    c2_name: String,
    server_process: rpc::ServerProcess,
) {
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

fn default_redirector_rules(
    _msg: C2GetRedirectorRuleMessage,
) -> C2GetRedirectorRuleMessageResponse {
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

// ===========================================================================
// Agent (Payload Type) container
// ===========================================================================

use constants::{
    PT_BUILD_RESPONSE_ROUTING_KEY, PT_CHECK_IF_CALLBACKS_ALIVE_ROUTING_KEY,
    PT_ON_NEW_CALLBACK_RESPONSE_ROUTING_KEY, PT_ON_NEW_CALLBACK_ROUTING_KEY,
    PT_PAYLOAD_BUILD_ROUTING_KEY, PT_TASK_COMPLETION_FUNCTION_ROUTING_KEY,
    PT_TASK_COMPLETION_RESPONSE_ROUTING_KEY, PT_TASK_CREATE_TASKING_RESPONSE_ROUTING_KEY,
    PT_TASK_CREATE_TASKING_ROUTING_KEY, PT_TASK_OPSEC_POST_RESPONSE_ROUTING_KEY,
    PT_TASK_OPSEC_POST_ROUTING_KEY, PT_TASK_OPSEC_PRE_RESPONSE_ROUTING_KEY,
    PT_TASK_OPSEC_PRE_ROUTING_KEY, PT_TASK_PROCESS_RESPONSE_RESPONSE_ROUTING_KEY,
    PT_TASK_PROCESS_RESPONSE_ROUTING_KEY,
};
use structs::{
    PTCheckIfCallbacksAliveMessage, PTCheckIfCallbacksAliveMessageResponse,
    PTOnNewCallbackAllData, PTOnNewCallbackResponse, PTTaskCompletionFunctionMessage,
    PTTaskCompletionFunctionMessageResponse, PTTaskCreateTaskingMessageResponse,
    PTTaskMessageAllData, PTTaskOPSECPostTaskMessageResponse, PTTTaskOPSECPreTaskMessageResponse,
    PtTaskProcessResponseMessage, PTTaskProcessResponseMessageResponse,
};

// ---------------------------------------------------------------------------
// Handler type aliases
// ---------------------------------------------------------------------------

pub type BuildHandler = fn(PayloadBuildMessage) -> PayloadBuildResponse;
pub type CallbacksAliveHandler =
    fn(PTCheckIfCallbacksAliveMessage) -> PTCheckIfCallbacksAliveMessageResponse;
pub type OnNewCallbackHandler = fn(PTOnNewCallbackAllData) -> PTOnNewCallbackResponse;
pub type CreateTaskingHandler = fn(PTTaskMessageAllData) -> PTTaskCreateTaskingMessageResponse;
pub type OpsecPreHandler = fn(PTTaskMessageAllData) -> PTTTaskOPSECPreTaskMessageResponse;
pub type OpsecPostHandler = fn(PTTaskMessageAllData) -> PTTaskOPSECPostTaskMessageResponse;
pub type ProcessResponseHandler =
    fn(PtTaskProcessResponseMessage) -> PTTaskProcessResponseMessageResponse;
pub type CompletionFunctionHandler =
    fn(PTTaskCompletionFunctionMessage) -> PTTaskCompletionFunctionMessageResponse;

// ---------------------------------------------------------------------------
// Per-command handler set
// ---------------------------------------------------------------------------

/// Handlers for a single command's task lifecycle.
#[derive(Clone)]
pub struct CommandHandlers {
    pub create_tasking: Option<CreateTaskingHandler>,
    pub opsec_pre: Option<OpsecPreHandler>,
    pub opsec_post: Option<OpsecPostHandler>,
    pub process_response: Option<ProcessResponseHandler>,
    pub completion_function: Option<CompletionFunctionHandler>,
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Builder for [`MythicAgentContainer`].
pub struct MythicAgentContainerBuilder {
    payload_type: Option<PayloadType>,
    commands: Vec<Command>,
    config: Option<RabbitMQConfig>,
    build_handler: Option<BuildHandler>,
    callbacks_alive_handler: Option<CallbacksAliveHandler>,
    on_new_callback_handler: Option<OnNewCallbackHandler>,
    command_handlers: HashMap<String, CommandHandlers>,
}

impl Default for MythicAgentContainerBuilder {
    fn default() -> Self {
        Environment::initialize();
        Self {
            payload_type: None,
            commands: Vec::new(),
            config: None,
            build_handler: None,
            callbacks_alive_handler: None,
            on_new_callback_handler: None,
            command_handlers: HashMap::new(),
        }
    }
}

impl MythicAgentContainerBuilder {
    pub fn payload_type(mut self, pt: PayloadType) -> Self {
        self.payload_type = Some(pt);
        self
    }

    pub fn commands(mut self, cmds: Vec<Command>) -> Self {
        self.commands = cmds;
        self
    }

    /// Override the RabbitMQ config. Defaults to [`RabbitMQConfig::from_env`].
    pub fn rabbitmq_config(mut self, config: RabbitMQConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Required: the payload build handler.
    pub fn on_build(mut self, f: BuildHandler) -> Self {
        self.build_handler = Some(f);
        self
    }

    pub fn on_callbacks_alive(mut self, f: CallbacksAliveHandler) -> Self {
        self.callbacks_alive_handler = Some(f);
        self
    }

    pub fn on_new_callback(mut self, f: OnNewCallbackHandler) -> Self {
        self.on_new_callback_handler = Some(f);
        self
    }

    /// Register per-command handlers.
    pub fn command_handlers(mut self, command_name: impl Into<String>, h: CommandHandlers) -> Self {
        self.command_handlers.insert(command_name.into(), h);
        self
    }

    pub fn build(self) -> MythicAgentContainer {
        MythicAgentContainer {
            payload_type: self.payload_type.expect("PayloadType is required"),
            commands: self.commands,
            config: self.config.unwrap_or_else(RabbitMQConfig::from_env),
            build_handler: self.build_handler.expect("build handler is required"),
            callbacks_alive_handler: self.callbacks_alive_handler,
            on_new_callback_handler: self.on_new_callback_handler,
            command_handlers: self.command_handlers,
        }
    }
}

// ---------------------------------------------------------------------------
// Main agent container type
// ---------------------------------------------------------------------------

/// Mirrors `MythicContainer.StartAndRunForever` from the Go library, scoped to
/// payload type (agent) services.
pub struct MythicAgentContainer {
    pub payload_type: PayloadType,
    pub commands: Vec<Command>,
    pub config: RabbitMQConfig,
    pub build_handler: BuildHandler,
    pub callbacks_alive_handler: Option<CallbacksAliveHandler>,
    pub on_new_callback_handler: Option<OnNewCallbackHandler>,
    pub command_handlers: HashMap<String, CommandHandlers>,
}

impl MythicAgentContainer {
    pub fn builder() -> MythicAgentContainerBuilder {
        MythicAgentContainerBuilder::default()
    }

    /// Connect to RabbitMQ, sync the payload type definition, then spawn all
    /// listener tasks and block forever.
    pub async fn start_and_run_forever(self) {
        let pt_name = self.payload_type.name.clone();
        let config = self.config.clone();

        // 1. Connect and sync - retry until Mythic acknowledges.
        loop {
            let conn = connection::connect_with_retry(&config).await;

            let sync_msg = agent_sync::build_pt_sync_message(
                self.payload_type.clone(),
                self.commands.clone(),
            );

            match agent_sync::send_pt_sync(&conn, sync_msg).await {
                Ok(()) => {
                    info!("Payload type '{}' synced. Spawning listeners...", pt_name);

                    // 2. Spawn all listeners.
                    self.spawn_pt_listeners(&pt_name, config.clone());

                    // 3. Block until Ctrl+C.
                    tokio::signal::ctrl_c()
                        .await
                        .expect("Failed to listen for Ctrl+C");

                    info!("Shutting down agent container for '{}'", pt_name);
                    return;
                }
                Err(e) => {
                    error!("PT sync failed: {}. Reconnecting...", e);
                    tokio::time::sleep(Duration::from_secs(RETRY_CONNECT_DELAY_SECS)).await;
                }
            }
        }
    }

    fn spawn_pt_listeners(&self, pt_name: &str, config: RabbitMQConfig) {
        // Build listener
        let build_key = get_routing_key(pt_name, PT_PAYLOAD_BUILD_ROUTING_KEY);
        let build_handler = self.build_handler;
        agent_rpc::spawn_pt_receiver::<PayloadBuildMessage, PayloadBuildResponse, _>(
            config.clone(),
            build_key,
            PT_BUILD_RESPONSE_ROUTING_KEY,
            build_handler,
        );

        // Callbacks alive listener (optional)
        let callbacks_alive_key =
            get_routing_key(pt_name, PT_CHECK_IF_CALLBACKS_ALIVE_ROUTING_KEY);
        let callbacks_alive_handler = self
            .callbacks_alive_handler
            .unwrap_or(default_callbacks_alive);
        agent_rpc::spawn_pt_receiver::<
            PTCheckIfCallbacksAliveMessage,
            PTCheckIfCallbacksAliveMessageResponse,
            _,
        >(
            config.clone(),
            callbacks_alive_key,
            // callbacks alive has no dedicated outbound key; reply_to is used by Mythic
            // but we use the pattern where we publish back to the inbound response key.
            // The Go library doesn't have a fixed response key for this one, so we
            // use the on_new_callback_response key as a safe default (noop in practice
            // when no handler is registered). Mythic ignores stray publishes.
            PT_ON_NEW_CALLBACK_RESPONSE_ROUTING_KEY,
            callbacks_alive_handler,
        );

        // On new callback listener (optional)
        let on_new_callback_key = get_routing_key(pt_name, PT_ON_NEW_CALLBACK_ROUTING_KEY);
        let on_new_callback_handler = self
            .on_new_callback_handler
            .unwrap_or(default_on_new_callback);
        agent_rpc::spawn_pt_receiver::<PTOnNewCallbackAllData, PTOnNewCallbackResponse, _>(
            config.clone(),
            on_new_callback_key,
            PT_ON_NEW_CALLBACK_RESPONSE_ROUTING_KEY,
            on_new_callback_handler,
        );

        // Per-command listeners
        for (cmd_name, handlers) in &self.command_handlers {
            // create_tasking
            if let Some(h) = handlers.create_tasking {
                let key = get_routing_key(pt_name, PT_TASK_CREATE_TASKING_ROUTING_KEY);
                agent_rpc::spawn_pt_receiver::<
                    PTTaskMessageAllData,
                    PTTaskCreateTaskingMessageResponse,
                    _,
                >(
                    config.clone(),
                    format!("{}_{}", key, cmd_name),
                    PT_TASK_CREATE_TASKING_RESPONSE_ROUTING_KEY,
                    h,
                );
            }

            // opsec_pre
            if let Some(h) = handlers.opsec_pre {
                let key = get_routing_key(pt_name, PT_TASK_OPSEC_PRE_ROUTING_KEY);
                agent_rpc::spawn_pt_receiver::<
                    PTTaskMessageAllData,
                    PTTTaskOPSECPreTaskMessageResponse,
                    _,
                >(
                    config.clone(),
                    format!("{}_{}", key, cmd_name),
                    PT_TASK_OPSEC_PRE_RESPONSE_ROUTING_KEY,
                    h,
                );
            }

            // opsec_post
            if let Some(h) = handlers.opsec_post {
                let key = get_routing_key(pt_name, PT_TASK_OPSEC_POST_ROUTING_KEY);
                agent_rpc::spawn_pt_receiver::<
                    PTTaskMessageAllData,
                    PTTaskOPSECPostTaskMessageResponse,
                    _,
                >(
                    config.clone(),
                    format!("{}_{}", key, cmd_name),
                    PT_TASK_OPSEC_POST_RESPONSE_ROUTING_KEY,
                    h,
                );
            }

            // process_response
            if let Some(h) = handlers.process_response {
                let key = get_routing_key(pt_name, PT_TASK_PROCESS_RESPONSE_ROUTING_KEY);
                agent_rpc::spawn_pt_receiver::<
                    PtTaskProcessResponseMessage,
                    PTTaskProcessResponseMessageResponse,
                    _,
                >(
                    config.clone(),
                    format!("{}_{}", key, cmd_name),
                    PT_TASK_PROCESS_RESPONSE_RESPONSE_ROUTING_KEY,
                    h,
                );
            }

            // completion_function
            if let Some(h) = handlers.completion_function {
                let key = get_routing_key(pt_name, PT_TASK_COMPLETION_FUNCTION_ROUTING_KEY);
                agent_rpc::spawn_pt_receiver::<
                    PTTaskCompletionFunctionMessage,
                    PTTaskCompletionFunctionMessageResponse,
                    _,
                >(
                    config.clone(),
                    format!("{}_{}", key, cmd_name),
                    PT_TASK_COMPLETION_RESPONSE_ROUTING_KEY,
                    h,
                );
            }
        }

        // Resync listener
        let resync_pt = self.payload_type.clone();
        let resync_commands = self.commands.clone();
        let resync_config = config.clone();
        let resync_name = pt_name.to_string();
        tokio::spawn(async move {
            agent_rpc::run_pt_resync_listener(
                resync_config,
                resync_name,
                resync_pt,
                resync_commands,
            )
            .await;
        });
    }
}

// ---------------------------------------------------------------------------
// Default (stub) agent handlers
// ---------------------------------------------------------------------------

fn default_callbacks_alive(
    msg: PTCheckIfCallbacksAliveMessage,
) -> PTCheckIfCallbacksAliveMessageResponse {
    PTCheckIfCallbacksAliveMessageResponse {
        success: true,
        error: String::new(),
        callbacks: msg
            .callbacks
            .into_iter()
            .map(|c| structs::PTCallbacksToCheckResponse {
                agent_callback_id: c.agent_callback_id,
                active: false,
            })
            .collect(),
    }
}

fn default_on_new_callback(msg: PTOnNewCallbackAllData) -> PTOnNewCallbackResponse {
    PTOnNewCallbackResponse {
        agent_callback_id: msg.callback.agent_callback_id,
        success: true,
        error: String::new(),
    }
}
