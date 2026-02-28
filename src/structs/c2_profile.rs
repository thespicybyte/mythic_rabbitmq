// Mirrors c2_structs/*.go from MythicMeta/MythicContainer
// JSON tags are kept identical to the Go source so Mythic can deserialize them.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// C2 Sync (sent once on startup to register the profile)
// ---------------------------------------------------------------------------

/// Top-level message sent to Mythic to register a C2 profile.
/// Mirrors C2SyncMessage in send_c2_sync_data.go
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SyncMessage {
    #[serde(rename = "c2_profile")]
    pub profile: C2ProfileDefinition,
    #[serde(rename = "parameters")]
    pub parameters: Vec<C2Parameter>,
    #[serde(rename = "container_version")]
    pub container_version: String,
}

/// Response from Mythic after attempting to sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SyncMessageResponse {
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
}

/// The C2 profile definition fields sent to Mythic over the wire.
/// Mirrors C2Profile in c2_sync.go - only fields with real json tags are included.
/// Fields tagged `json:"-"` in Go (ServerBinaryPath, ServerFolderPath, all function
/// pointers) are NOT serialized and are kept out of this struct entirely.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2ProfileDefinition {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "author")]
    pub author: String,
    #[serde(rename = "is_p2p")]
    pub is_p2p: bool,
    #[serde(rename = "is_server_routed")]
    pub is_server_routed: bool,
    #[serde(rename = "semver")]
    pub semver: String,
    #[serde(rename = "agent_icon")]
    pub agent_icon: Option<Vec<u8>>,
    #[serde(rename = "dark_mode_agent_icon")]
    pub dark_mode_agent_icon: Option<Vec<u8>>,
}

// ---------------------------------------------------------------------------
// C2 Parameter
// ---------------------------------------------------------------------------

/// Mirrors C2Parameter in c2_sync.go
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2Parameter {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "default_value")]
    pub default_value: serde_json::Value,
    #[serde(rename = "randomize")]
    pub randomize: bool,
    #[serde(rename = "format_string")]
    pub format_string: String,
    #[serde(rename = "parameter_type")]
    pub parameter_type: String,
    #[serde(rename = "required")]
    pub required: bool,
    #[serde(rename = "verifier_regex")]
    pub verifier_regex: String,
    #[serde(rename = "crypto_type")]
    pub is_crypto_type: bool,
    #[serde(rename = "choices")]
    pub choices: Vec<String>,
    #[serde(rename = "dictionary_choices")]
    pub dictionary_choices: Vec<C2ParameterDictionary>,
    #[serde(rename = "ui_position")]
    pub ui_position: u32,
}

/// Mirrors C2ParameterDictionary in c2_sync.go
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2ParameterDictionary {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "default_value")]
    pub default_value: String,
    #[serde(rename = "required")]
    pub required: bool,
    #[serde(rename = "crypto_type")]
    pub is_crypto_type: bool,
    #[serde(rename = "description")]
    pub description: String,
}

/// Well-known parameter type strings (mirrors constants in c2_sync.go)
pub mod parameter_type {
    pub const STRING: &str = "String";
    pub const BOOLEAN: &str = "Boolean";
    pub const CHOOSE_ONE: &str = "ChooseOne";
    pub const CHOOSE_ONE_CUSTOM: &str = "ChooseOneCustom";
    pub const CHOOSE_MULTIPLE: &str = "ChooseMultiple";
    pub const ARRAY: &str = "Array";
    pub const TYPED_ARRAY: &str = "TypedArray";
    pub const DATE: &str = "Date";
    pub const DICTIONARY: &str = "Dictionary";
    pub const NUMBER: &str = "Number";
    pub const FILE: &str = "File";
    pub const FILE_MULTIPLE: &str = "FileMultiple";
}

// ---------------------------------------------------------------------------
// C2 Status Update (container -> Mythic)
// ---------------------------------------------------------------------------

/// Sent to Mythic after sync and after any start/stop server event.
/// Mirrors MythicRPCC2UpdateStatusMessage in send_c2_rpc_update_status.go.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MythicRPCC2UpdateStatusMessage {
    #[serde(rename = "c2_profile")]
    pub c2_profile: String,
    #[serde(rename = "server_running")]
    pub server_running: bool,
    #[serde(rename = "error")]
    pub error: String,
}

// ---------------------------------------------------------------------------
// Shared RPC base - the payload Mythic sends to each RPC queue
// ---------------------------------------------------------------------------

/// The parameters block included in every inbound C2 RPC call.
/// Mirrors C2Parameters in c2_structs/utils.go
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2Parameters {
    #[serde(rename = "c2_profile_name")]
    pub c2_profile_name: String,
    #[serde(rename = "parameters")]
    pub parameters: HashMap<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Config Check
// ---------------------------------------------------------------------------

/// Mirrors C2ConfigCheckMessage (embeds C2Parameters)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2ConfigCheckMessage {
    #[serde(flatten)]
    pub c2_parameters: C2Parameters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2ConfigCheckMessageResponse {
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
    #[serde(rename = "message")]
    pub message: String,
    #[serde(rename = "restart_internal_server")]
    pub restart_internal_server: bool,
}

// ---------------------------------------------------------------------------
// OPSEC Check
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2OPSECMessage {
    #[serde(flatten)]
    pub c2_parameters: C2Parameters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2OPSECMessageResponse {
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
    #[serde(rename = "message")]
    pub message: String,
    #[serde(rename = "restart_internal_server")]
    pub restart_internal_server: bool,
}

// ---------------------------------------------------------------------------
// Get IOC
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2GetIOCMessage {
    #[serde(flatten)]
    pub c2_parameters: C2Parameters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IOC {
    #[serde(rename = "type")]
    pub ioc_type: String,
    #[serde(rename = "ioc")]
    pub ioc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2GetIOCMessageResponse {
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
    #[serde(rename = "iocs")]
    pub iocs: Vec<IOC>,
    #[serde(rename = "restart_internal_server")]
    pub restart_internal_server: bool,
}

// ---------------------------------------------------------------------------
// Sample Message
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SampleMessageMessage {
    #[serde(flatten)]
    pub c2_parameters: C2Parameters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SampleMessageResponse {
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
    #[serde(rename = "message")]
    pub message: String,
    #[serde(rename = "restart_internal_server")]
    pub restart_internal_server: bool,
}

// ---------------------------------------------------------------------------
// Redirector Rules
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2GetRedirectorRuleMessage {
    #[serde(flatten)]
    pub c2_parameters: C2Parameters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2GetRedirectorRuleMessageResponse {
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
    #[serde(rename = "message")]
    pub message: String,
    #[serde(rename = "restart_internal_server")]
    pub restart_internal_server: bool,
}

// ---------------------------------------------------------------------------
// Host File
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2HostFileMessage {
    #[serde(rename = "c2_profile_name")]
    pub c2_profile_name: String,
    #[serde(rename = "file_uuid")]
    pub file_uuid: String,
    #[serde(rename = "host_url")]
    pub host_url: String,
    #[serde(rename = "remove")]
    pub remove: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2HostFileMessageResponse {
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
    #[serde(rename = "restart_internal_server")]
    pub restart_internal_server: bool,
}
