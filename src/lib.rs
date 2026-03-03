//! # mythic-rabbitmq
//!
//! Rust port of the RabbitMQ layer from [MythicMeta/MythicContainer](https://github.com/MythicMeta/MythicContainer).
//!
//! Handles C2 profile and payload type (agent) registration and RPC dispatch
//! for Mythic C2 containers.
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

pub mod agent_container;
pub mod agent_rpc;
pub mod agent_sync;
pub mod c2_profile;
pub mod connection;
pub mod constants;
mod environment;
pub mod error;
pub mod rpc;
pub mod structs;
pub mod sync;

pub use connection::RabbitMQConfig;
pub use error::{MythicError, Result};

// C2 re-exports
pub use c2_profile::{
    ConfigCheckHandler, GetIocHandler, HostFileHandler, MythicC2Container,
    MythicC2ContainerBuilder, OpsecCheckHandler, RedirectorRulesHandler, SampleMessageHandler,
};
pub use structs::{
    C2ConfigCheckMessage, C2ConfigCheckMessageResponse, C2GetIOCMessage, C2GetIOCMessageResponse,
    C2GetRedirectorRuleMessage, C2GetRedirectorRuleMessageResponse, C2HostFileMessage,
    C2HostFileMessageResponse, C2OPSECMessage, C2OPSECMessageResponse, C2Parameter,
    C2ParameterDictionary, C2ProfileDefinition, C2SampleMessageMessage, C2SampleMessageResponse,
    C2SyncMessage, C2SyncMessageResponse, IOC, MythicRPCC2UpdateStatusMessage,
};

// Agent re-exports
pub use agent_container::{
    BoxFuture, BuildHandler, CallbacksAliveHandler, CommandHandlers, CompletionFunctionHandler,
    CreateTaskingHandler, MythicAgentContainer, MythicAgentContainerBuilder,
    OnNewCallbackHandler, OpsecPostHandler, OpsecPreHandler, ProcessResponseHandler,
};
pub use agent_sync::send_payload_update_build_step;
pub use structs::{
    AgentType, BuildParameter, BuildStep, Command, CommandAttribute, CommandParameter,
    MessageFormat, MythicRPCPayloadUpdateBuildStepMessage,
    MythicRPCPayloadUpdateBuildStepMessageResponse, PTTaskCreateTaskingMessageResponse,
    PTTaskMessageAllData, PayloadBuildMessage, PayloadBuildResponse, PayloadType,
    PayloadTypeSyncMessage,
};
