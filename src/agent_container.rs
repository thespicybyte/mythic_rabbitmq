use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

/// A heap-allocated, Send future — used as the return type for async build handlers.
/// Define your build function as:
/// ```rust
/// fn build(msg: PayloadBuildMessage) -> BoxFuture<PayloadBuildResponse> {
///     Box::pin(async move { /* ... */ })
/// }
/// ```
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send + 'static>>;

use crate::agent_rpc;
use crate::agent_sync;
use crate::connection::RabbitMQConfig;
use crate::constants::{
    get_routing_key, PT_BUILD_RESPONSE_ROUTING_KEY, PT_CHECK_IF_CALLBACKS_ALIVE_ROUTING_KEY,
    PT_ON_NEW_CALLBACK_RESPONSE_ROUTING_KEY, PT_ON_NEW_CALLBACK_ROUTING_KEY,
    PT_PAYLOAD_BUILD_ROUTING_KEY, PT_TASK_COMPLETION_FUNCTION_ROUTING_KEY,
    PT_TASK_COMPLETION_RESPONSE_ROUTING_KEY, PT_TASK_CREATE_TASKING_RESPONSE_ROUTING_KEY,
    PT_TASK_CREATE_TASKING_ROUTING_KEY, PT_TASK_OPSEC_POST_RESPONSE_ROUTING_KEY,
    PT_TASK_OPSEC_POST_ROUTING_KEY, PT_TASK_OPSEC_PRE_RESPONSE_ROUTING_KEY,
    PT_TASK_OPSEC_PRE_ROUTING_KEY, PT_TASK_PROCESS_RESPONSE_RESPONSE_ROUTING_KEY,
    PT_TASK_PROCESS_RESPONSE_ROUTING_KEY, RETRY_CONNECT_DELAY_SECS,
};
use crate::environment::Environment;
use crate::structs::{
    Command, PTCallbacksToCheckResponse, PTCheckIfCallbacksAliveMessage,
    PTCheckIfCallbacksAliveMessageResponse, PTOnNewCallbackAllData, PTOnNewCallbackResponse,
    PTTTaskOPSECPreTaskMessageResponse, PTTaskCompletionFunctionMessage,
    PTTaskCompletionFunctionMessageResponse, PTTaskCreateTaskingMessageResponse,
    PTTaskMessageAllData, PTTaskOPSECPostTaskMessageResponse, PTTaskProcessResponseMessageResponse,
    PayloadBuildMessage, PayloadBuildResponse, PayloadType, PtTaskProcessResponseMessage,
};

// ---------------------------------------------------------------------------
// Handler type aliases
// ---------------------------------------------------------------------------

pub type BuildHandler = fn(PayloadBuildMessage) -> BoxFuture<PayloadBuildResponse>;
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

impl Default for CommandHandlers {
    fn default() -> CommandHandlers {
        Self {
            create_tasking: Some(default_create_tasking),
            opsec_pre: Some(default_opsec_pre),
            opsec_post: Some(default_opsec_post),
            process_response: Some(default_process_response),
            completion_function: Some(default_completion_function)
        }

    }
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

        // Store config globally so mythicrpc send functions can use it without
        // the caller needing to manage connections (mirrors Go's RabbitMQConnection).
        crate::connection::set_global_config(config.clone());

        // 1. Connect and sync - retry until Mythic acknowledges.
        loop {
            let conn = crate::connection::connect_with_retry(&config).await;

            let sync_msg =
                agent_sync::build_pt_sync_message(self.payload_type.clone(), self.commands.clone());

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
        // Build listener — async so handlers can call mythicrpc send functions.
        let build_key = get_routing_key(pt_name, PT_PAYLOAD_BUILD_ROUTING_KEY);
        let build_handler = self.build_handler;
        agent_rpc::spawn_pt_receiver_async::<PayloadBuildMessage, PayloadBuildResponse, _, _>(
            config.clone(),
            build_key,
            PT_BUILD_RESPONSE_ROUTING_KEY,
            move |msg| build_handler(msg),
        );

        // Callbacks alive listener (optional)
        let callbacks_alive_key = get_routing_key(pt_name, PT_CHECK_IF_CALLBACKS_ALIVE_ROUTING_KEY);
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

        // Build dispatch maps: command_name -> handler.
        // Mythic routes ALL tasks of a given lifecycle phase to a single queue
        // (e.g. "mynoxide_pt_task_opsec_pre_check"), not a per-command queue.
        // We subscribe to the base key and dispatch internally by command_name.
        let create_tasking_map: Arc<HashMap<String, CreateTaskingHandler>> = Arc::new(
            self.command_handlers
                .iter()
                .filter_map(|(name, h)| h.create_tasking.map(|f| (name.clone(), f)))
                .collect(),
        );
        let opsec_pre_map: Arc<HashMap<String, OpsecPreHandler>> = Arc::new(
            self.command_handlers
                .iter()
                .filter_map(|(name, h)| h.opsec_pre.map(|f| (name.clone(), f)))
                .collect(),
        );
        let opsec_post_map: Arc<HashMap<String, OpsecPostHandler>> = Arc::new(
            self.command_handlers
                .iter()
                .filter_map(|(name, h)| h.opsec_post.map(|f| (name.clone(), f)))
                .collect(),
        );
        let process_response_map: Arc<HashMap<String, ProcessResponseHandler>> = Arc::new(
            self.command_handlers
                .iter()
                .filter_map(|(name, h)| h.process_response.map(|f| (name.clone(), f)))
                .collect(),
        );
        let completion_function_map: Arc<HashMap<String, CompletionFunctionHandler>> = Arc::new(
            self.command_handlers
                .iter()
                .filter_map(|(name, h)| h.completion_function.map(|f| (name.clone(), f)))
                .collect(),
        );

        // create_tasking — one queue, dispatch by command_name
        {
            let key = get_routing_key(pt_name, PT_TASK_CREATE_TASKING_ROUTING_KEY);
            let map = create_tasking_map.clone();
            agent_rpc::spawn_pt_receiver::<
                PTTaskMessageAllData,
                PTTaskCreateTaskingMessageResponse,
                _,
            >(
                config.clone(),
                key,
                PT_TASK_CREATE_TASKING_RESPONSE_ROUTING_KEY,
                move |msg: PTTaskMessageAllData| {
                    if let Some(h) = map.get(&msg.task.command_name) {
                        h(msg)
                    } else {
                        default_create_tasking(msg)
                    }
                },
            );
        }

        // opsec_pre — one queue, dispatch by command_name
        {
            let key = get_routing_key(pt_name, PT_TASK_OPSEC_PRE_ROUTING_KEY);
            let map = opsec_pre_map.clone();
            agent_rpc::spawn_pt_receiver::<
                PTTaskMessageAllData,
                PTTTaskOPSECPreTaskMessageResponse,
                _,
            >(
                config.clone(),
                key,
                PT_TASK_OPSEC_PRE_RESPONSE_ROUTING_KEY,
                move |msg: PTTaskMessageAllData| {
                    if let Some(h) = map.get(&msg.task.command_name) {
                        h(msg)
                    } else {
                        default_opsec_pre(msg)
                    }
                },
            );
        }

        // opsec_post — one queue, dispatch by command_name
        {
            let key = get_routing_key(pt_name, PT_TASK_OPSEC_POST_ROUTING_KEY);
            let map = opsec_post_map.clone();
            agent_rpc::spawn_pt_receiver::<
                PTTaskMessageAllData,
                PTTaskOPSECPostTaskMessageResponse,
                _,
            >(
                config.clone(),
                key,
                PT_TASK_OPSEC_POST_RESPONSE_ROUTING_KEY,
                move |msg: PTTaskMessageAllData| {
                    if let Some(h) = map.get(&msg.task.command_name) {
                        h(msg)
                    } else {
                        default_opsec_post(msg)
                    }
                },
            );
        }

        // process_response — one queue, dispatch by command_name
        {
            let key = get_routing_key(pt_name, PT_TASK_PROCESS_RESPONSE_ROUTING_KEY);
            let map = process_response_map.clone();
            agent_rpc::spawn_pt_receiver::<
                PtTaskProcessResponseMessage,
                PTTaskProcessResponseMessageResponse,
                _,
            >(
                config.clone(),
                key,
                PT_TASK_PROCESS_RESPONSE_RESPONSE_ROUTING_KEY,
                move |msg: PtTaskProcessResponseMessage| {
                    if let Some(h) = map.get(&msg.task.command_name) {
                        h(msg)
                    } else {
                        default_process_response(msg)
                    }
                },
            );
        }

        // completion_function — one queue, dispatch by command_name
        {
            let key = get_routing_key(pt_name, PT_TASK_COMPLETION_FUNCTION_ROUTING_KEY);
            let map = completion_function_map.clone();
            agent_rpc::spawn_pt_receiver::<
                PTTaskCompletionFunctionMessage,
                PTTaskCompletionFunctionMessageResponse,
                _,
            >(
                config.clone(),
                key,
                PT_TASK_COMPLETION_RESPONSE_ROUTING_KEY,
                move |msg: PTTaskCompletionFunctionMessage| {
                    if let Some(h) = map.get(&msg.task.command_name) {
                        h(msg)
                    } else {
                        default_completion_function(msg)
                    }
                },
            );
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
            .map(|c| PTCallbacksToCheckResponse {
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

pub fn default_create_tasking(msg: PTTaskMessageAllData) -> PTTaskCreateTaskingMessageResponse {
    PTTaskCreateTaskingMessageResponse {
        task_id: msg.task.id,
        success: true,
        error: String::new(),
        command_name: msg.task.command_name,
        params: msg.task.params,
        display_params: String::new(),
        stdout: String::new(),
        stderr: String::new(),
        completed: false,
        status: String::new(),
    }
}

pub fn default_opsec_pre(msg: PTTaskMessageAllData) -> PTTTaskOPSECPreTaskMessageResponse {
    PTTTaskOPSECPreTaskMessageResponse {
        task_id: msg.task.id,
        success: true,
        error: String::new(),
        opsec_pre_blocked: false,
        opsec_pre_message: String::new(),
        opsec_pre_bypassed: false,
        opsec_pre_bypass_role: String::new(),
    }
}

pub fn default_opsec_post(msg: PTTaskMessageAllData) -> PTTaskOPSECPostTaskMessageResponse {
    PTTaskOPSECPostTaskMessageResponse {
        task_id: msg.task.id,
        success: true,
        error: String::new(),
        opsec_post_blocked: false,
        opsec_post_message: String::new(),
        opsec_post_bypassed: false,
        opsec_post_bypass_role: String::new(),
    }
}

pub fn default_process_response(
    msg: PtTaskProcessResponseMessage,
) -> PTTaskProcessResponseMessageResponse {
    PTTaskProcessResponseMessageResponse {
        task_id: msg.task.id,
        success: true,
        error: String::new(),
    }
}

pub fn default_completion_function(
    msg: PTTaskCompletionFunctionMessage,
) -> PTTaskCompletionFunctionMessageResponse {
    PTTaskCompletionFunctionMessageResponse {
        task_id: msg.task.id,
        success: true,
        error: String::new(),
        params: String::new(),
        status: String::new(),
        display_params: String::new(),
        stdout: String::new(),
        stderr: String::new(),
        completed: false,
    }
}
