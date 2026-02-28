// Mirrors agent_structs/*.go from MythicMeta/MythicContainer
// JSON tags are kept identical to the Go source so Mythic can deserialize them.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// OS / UI Feature constants (from constants.go)
// ---------------------------------------------------------------------------

pub const SUPPORTED_OS_LINUX: &str = "Linux";
pub const SUPPORTED_OS_MACOS: &str = "macOS";
pub const SUPPORTED_OS_WINDOWS: &str = "Windows";
pub const SUPPORTED_OS_CHROME: &str = "Chrome";
pub const SUPPORTED_OS_ANDROID: &str = "Android";
pub const SUPPORTED_OS_IOS: &str = "iOS";
pub const SUPPORTED_OS_SMART_TV: &str = "SmartTV";
pub const SUPPORTED_OS_FREEBSD: &str = "FreeBSD";

pub const SUPPORTED_UI_FEATURE_TASK_RESPONSE_INTERACTIVE: &str = "task_response:interactive";
pub const SUPPORTED_UI_FEATURE_FILE_BROWSER_ROOT: &str = "file_browser:list_files";
pub const SUPPORTED_UI_FEATURE_PROCESS_LIST: &str = "process_browser:list";

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    Agent,
    Wrapper,
    Service,
    #[serde(rename = "command_augment")]
    CommandAugment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageFormat {
    Json,
    Xml,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HideConditionOperand {
    #[serde(rename = "eq")]
    Eq,
    #[serde(rename = "neq")]
    Neq,
    #[serde(rename = "in")]
    In,
    #[serde(rename = "nin")]
    Nin,
    #[serde(rename = "lt")]
    Lt,
    #[serde(rename = "gt")]
    Gt,
    #[serde(rename = "lte")]
    Lte,
    #[serde(rename = "gte")]
    Gte,
    #[serde(rename = "sw")]
    Sw,
    #[serde(rename = "ew")]
    Ew,
    #[serde(rename = "co")]
    Co,
    #[serde(rename = "nco")]
    Nco,
}

/// OPSEC role string type.
pub type OpsecRole = String;
pub const OPSEC_ROLE_LEAD: &str = "lead";
pub const OPSEC_ROLE_OPERATOR: &str = "operator";
pub const OPSEC_ROLE_OTHER_OPERATOR: &str = "other_operator";

// ---------------------------------------------------------------------------
// BuildParameterType constants
// ---------------------------------------------------------------------------

pub mod build_parameter_type {
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
    pub const NONE: &str = "None";
}

// ---------------------------------------------------------------------------
// CommandParameterType constants
// ---------------------------------------------------------------------------

pub mod command_parameter_type {
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
    pub const CREDENTIAL_JSON: &str = "CredentialJson";
    pub const PAYLOAD_LIST: &str = "PayloadList";
    pub const AGENT_CONNECTED: &str = "AgentConnected";
    pub const CONNECTION_INFO: &str = "ConnectionInfo";
    pub const LINK_INFO: &str = "LinkInfo";
}

// ---------------------------------------------------------------------------
// Payload Type Sync (from structs_payload_sync.go)
// ---------------------------------------------------------------------------

/// Top-level sync message sent to Mythic to register a payload type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadTypeSyncMessage {
    #[serde(rename = "payload_type")]
    pub payload_type: PayloadType,
    #[serde(rename = "commands")]
    pub commands: Vec<Command>,
    #[serde(rename = "container_version")]
    pub container_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadTypeSyncMessageResponse {
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
}

/// Payload type definition (serializable fields only; function pointers excluded).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadType {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "file_extension")]
    pub file_extension: String,
    #[serde(rename = "author")]
    pub author: String,
    #[serde(rename = "supported_os")]
    pub supported_os: Vec<String>,
    #[serde(rename = "wrapper")]
    pub wrapper: bool,
    #[serde(rename = "wrapped_payloads")]
    pub wrapped_payloads: Vec<String>,
    #[serde(rename = "note")]
    pub note: String,
    #[serde(rename = "supports_dynamic_loading")]
    pub supports_dynamic_loading: bool,
    #[serde(rename = "build_parameters")]
    pub build_parameters: Vec<BuildParameter>,
    #[serde(rename = "build_steps")]
    pub build_steps: Vec<BuildStep>,
    #[serde(rename = "message_format")]
    pub message_format: MessageFormat,
    #[serde(rename = "agent_type")]
    pub agent_type: AgentType,
    #[serde(rename = "c2_profiles")]
    pub c2_profiles: Vec<PayloadBuildC2ProfileMessage>,
    #[serde(rename = "mythic_encrypts")]
    pub mythic_encrypts: bool,
    #[serde(rename = "translation_container_name")]
    pub translation_container_name: String,
    #[serde(rename = "agent_icon")]
    pub agent_icon: Option<Vec<u8>>,
    #[serde(rename = "custom_rpc_functions")]
    pub custom_rpc_functions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "needs_admin_permissions")]
    pub needs_admin_permissions: bool,
    #[serde(rename = "help_cmd")]
    pub help_cmd: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "version")]
    pub version: u32,
    #[serde(rename = "supported_ui_features")]
    pub supported_ui_features: Vec<String>,
    #[serde(rename = "author")]
    pub author: String,
    #[serde(rename = "mitre_attack_techniques")]
    pub mitre_attack_techniques: Vec<String>,
    #[serde(rename = "attributes")]
    pub attributes: CommandAttribute,
    #[serde(rename = "script_only")]
    pub script_only: bool,
    #[serde(rename = "parameters")]
    pub parameters: Vec<CommandParameter>,
    #[serde(rename = "browser_script")]
    pub browser_script: Option<BrowserScript>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandParameter {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "display_name")]
    pub display_name: String,
    #[serde(rename = "cli_name")]
    pub cli_name: String,
    #[serde(rename = "parameter_type")]
    pub parameter_type: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "choices")]
    pub choices: Vec<String>,
    #[serde(rename = "default_value")]
    pub default_value: Value,
    #[serde(rename = "required")]
    pub required: bool,
    #[serde(rename = "verifier_regex")]
    pub verifier_regex: String,
    #[serde(rename = "supported_agents")]
    pub supported_agents: Vec<String>,
    #[serde(rename = "supported_agent_build_parameters")]
    pub supported_agent_build_parameters: HashMap<String, String>,
    #[serde(rename = "choices_are_all_commands")]
    pub choices_are_all_commands: bool,
    #[serde(rename = "choices_are_loaded_commands")]
    pub choices_are_loaded_commands: bool,
    #[serde(rename = "filter_command_choices_by_supported_os")]
    pub filter_command_choices_by_supported_os: bool,
    #[serde(rename = "dynamic_query_function")]
    pub dynamic_query_function: Option<String>,
    #[serde(rename = "typedarray_parse_function")]
    pub typedarray_parse_function: Option<String>,
    #[serde(rename = "parameter_group_info")]
    pub parameter_group_info: Vec<ParameterGroupInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterGroupInfo {
    #[serde(rename = "group_name")]
    pub group_name: String,
    #[serde(rename = "parameter_is_required")]
    pub parameter_is_required: bool,
    #[serde(rename = "ui_position")]
    pub ui_position: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandAttribute {
    #[serde(rename = "filter_by_build_parameter")]
    pub filter_by_build_parameter: HashMap<String, String>,
    #[serde(rename = "supported_os")]
    pub supported_os: Vec<String>,
    #[serde(rename = "builtin")]
    pub builtin: bool,
    #[serde(rename = "suggested_command")]
    pub suggested_command: bool,
    #[serde(rename = "load_only")]
    pub load_only: bool,
    #[serde(rename = "script_only")]
    pub script_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserScript {
    #[serde(rename = "script")]
    pub script: String,
    #[serde(rename = "author")]
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildParameter {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "required")]
    pub required: bool,
    #[serde(rename = "verifier_regex")]
    pub verifier_regex: String,
    #[serde(rename = "default_value")]
    pub default_value: Value,
    #[serde(rename = "parameter_type")]
    pub parameter_type: String,
    #[serde(rename = "choices")]
    pub choices: Vec<String>,
    #[serde(rename = "randomize")]
    pub randomize: bool,
    #[serde(rename = "format_string")]
    pub format_string: String,
    #[serde(rename = "crypto_type")]
    pub is_crypto_type: bool,
    #[serde(rename = "hide_conditions")]
    pub hide_conditions: Vec<BuildParameterHideCondition>,
    #[serde(rename = "dictionary_choices")]
    pub dictionary_choices: Vec<BuildParameterDictionary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildParameterHideCondition {
    #[serde(rename = "parameter_name")]
    pub parameter_name: String,
    #[serde(rename = "operand")]
    pub operand: HideConditionOperand,
    #[serde(rename = "value")]
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildParameterDictionary {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildStep {
    #[serde(rename = "step_name")]
    pub step_name: String,
    #[serde(rename = "step_description")]
    pub step_description: String,
}

/// C2ParameterDictionary for the PT side (mirrors agent_structs version).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentC2ParameterDictionary {
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

/// C2ParameterDeviation describes a mismatch between what this payload type
/// expects from a C2 profile and what the C2 profile actually provides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2ParameterDeviation {
    #[serde(rename = "c2_profile_name")]
    pub c2_profile_name: String,
    #[serde(rename = "parameter_name")]
    pub parameter_name: String,
    #[serde(rename = "description")]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadBuildC2ProfileMessage {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "parameters")]
    pub parameters: HashMap<String, Value>,
    #[serde(rename = "crypto_params")]
    pub crypto_params: Vec<AgentC2ParameterDictionary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadBuildC2ProfileMessageResponse {
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
}

/// Payload configuration included in build messages from Mythic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadConfiguration {
    #[serde(rename = "payload_type")]
    pub payload_type: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "tag")]
    pub tag: String,
    #[serde(rename = "uuid")]
    pub uuid: String,
    #[serde(rename = "os")]
    pub os: String,
    #[serde(rename = "c2_profiles")]
    pub c2_profiles: Vec<PayloadConfigurationC2Profile>,
    #[serde(rename = "build_parameters")]
    pub build_parameters: Vec<PayloadConfigurationBuildParameter>,
    #[serde(rename = "commands")]
    pub commands: Vec<String>,
    #[serde(rename = "wrapped_payload")]
    pub wrapped_payload: String,
    #[serde(rename = "selected_os")]
    pub selected_os: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadConfigurationC2Profile {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "parameters")]
    pub parameters: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadConfigurationBuildParameter {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "value")]
    pub value: Value,
}

/// Message for other-service RPC calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCOtherServiceRPCMessage {
    #[serde(rename = "service_name")]
    pub service_name: String,
    #[serde(rename = "service_rpc_function")]
    pub service_rpc_function: String,
    #[serde(rename = "task_id")]
    pub task_id: u64,
    #[serde(rename = "message")]
    pub message: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCOtherServiceRPCMessageResponse {
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
    #[serde(rename = "result")]
    pub result: HashMap<String, Value>,
}

// ---------------------------------------------------------------------------
// Build structs (from structs_payload_build.go)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadBuildMessage {
    #[serde(rename = "payload_type")]
    pub payload_type: String,
    #[serde(rename = "filename")]
    pub filename: String,
    #[serde(rename = "tag")]
    pub tag: String,
    #[serde(rename = "selected_os")]
    pub selected_os: String,
    #[serde(rename = "commands")]
    pub commands: Vec<String>,
    #[serde(rename = "build_parameters")]
    pub build_parameters: BuildParameters,
    #[serde(rename = "c2_profiles")]
    pub c2_profiles: Vec<PayloadBuildC2Profile>,
    #[serde(rename = "wrapped_payload")]
    pub wrapped_payload: String,
    #[serde(rename = "wrapped_payload_uuid")]
    pub wrapped_payload_uuid: String,
    #[serde(rename = "uuid")]
    pub uuid: String,
    #[serde(rename = "all_c2_info")]
    pub all_c2_info: Vec<PayloadBuildC2ProfileMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadBuildC2Profile {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "parameters")]
    pub parameters: HashMap<String, Value>,
    #[serde(rename = "crypto_params")]
    pub crypto_params: Vec<CryptoArg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoArg {
    #[serde(rename = "enc_key")]
    pub enc_key: Option<String>,
    #[serde(rename = "dec_key")]
    pub dec_key: Option<String>,
    #[serde(rename = "type")]
    pub crypto_type: String,
    #[serde(rename = "value")]
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildParameters(pub HashMap<String, Value>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadBuildResponse {
    #[serde(rename = "uuid")]
    pub uuid: String,
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
    #[serde(rename = "payload")]
    pub payload: Option<Vec<u8>>,
    #[serde(rename = "status")]
    pub status: String,
    #[serde(rename = "build_message")]
    pub build_message: String,
    #[serde(rename = "build_stderr")]
    pub build_stderr: String,
    #[serde(rename = "build_stdout")]
    pub build_stdout: String,
    #[serde(rename = "updated_command_list")]
    pub updated_command_list: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Task message structs (from structs_task_messages.go)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTTaskMessageAllData {
    #[serde(rename = "task")]
    pub task: PTTaskMessageTaskData,
    #[serde(rename = "callback")]
    pub callback: PTTaskMessageCallbackData,
    #[serde(rename = "payload")]
    pub payload: PTTaskMessagePayloadData,
    #[serde(rename = "build_parameters")]
    pub build_parameters: Vec<PayloadConfigurationBuildParameter>,
    #[serde(rename = "commands")]
    pub commands: Vec<String>,
    #[serde(rename = "c2info")]
    pub c2info: Vec<PayloadBuildC2Profile>,
    #[serde(rename = "subtasks")]
    pub subtasks: Vec<PTTaskMessageTaskData>,
    #[serde(rename = "subtask_groups")]
    pub subtask_groups: Vec<SubtaskGroupName>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTTaskMessageTaskData {
    #[serde(rename = "id")]
    pub id: u64,
    #[serde(rename = "display_id")]
    pub display_id: u64,
    #[serde(rename = "agent_task_id")]
    pub agent_task_id: String,
    #[serde(rename = "command_name")]
    pub command_name: String,
    #[serde(rename = "params")]
    pub params: String,
    #[serde(rename = "timestamp")]
    pub timestamp: f64,
    #[serde(rename = "callback_id")]
    pub callback_id: u64,
    #[serde(rename = "status")]
    pub status: String,
    #[serde(rename = "original_params")]
    pub original_params: String,
    #[serde(rename = "display_params")]
    pub display_params: String,
    #[serde(rename = "comment")]
    pub comment: String,
    #[serde(rename = "stdout")]
    pub stdout: String,
    #[serde(rename = "stderr")]
    pub stderr: String,
    #[serde(rename = "completed")]
    pub completed: bool,
    #[serde(rename = "operator_username")]
    pub operator_username: String,
    #[serde(rename = "opsec_pre_bypassed")]
    pub opsec_pre_bypassed: bool,
    #[serde(rename = "opsec_post_bypassed")]
    pub opsec_post_bypassed: bool,
    #[serde(rename = "opsec_pre_bypass_role")]
    pub opsec_pre_bypass_role: OpsecRole,
    #[serde(rename = "opsec_post_bypass_role")]
    pub opsec_post_bypass_role: OpsecRole,
    #[serde(rename = "tags")]
    pub tags: Vec<String>,
    #[serde(rename = "token")]
    pub token: Option<Value>,
    #[serde(rename = "subtask_callback_function")]
    pub subtask_callback_function: String,
    #[serde(rename = "group_callback_function")]
    pub group_callback_function: String,
    #[serde(rename = "completed_callback_function")]
    pub completed_callback_function: String,
    #[serde(rename = "subtask_group_name")]
    pub subtask_group_name: String,
    #[serde(rename = "tasking_location")]
    pub tasking_location: String,
    #[serde(rename = "parameter_group_name")]
    pub parameter_group_name: String,
    #[serde(rename = "args")]
    pub args: PTTaskMessageArgsData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTTaskMessageArgsData {
    #[serde(rename = "command_line")]
    pub command_line: String,
    #[serde(rename = "tasking_location")]
    pub tasking_location: String,
    #[serde(rename = "parameter_group_name")]
    pub parameter_group_name: String,
    #[serde(flatten)]
    pub raw_params: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTTaskMessageCallbackData {
    #[serde(rename = "id")]
    pub id: u64,
    #[serde(rename = "display_id")]
    pub display_id: u64,
    #[serde(rename = "agent_callback_id")]
    pub agent_callback_id: String,
    #[serde(rename = "init_callback")]
    pub init_callback: f64,
    #[serde(rename = "last_checkin")]
    pub last_checkin: f64,
    #[serde(rename = "user")]
    pub user: String,
    #[serde(rename = "host")]
    pub host: String,
    #[serde(rename = "pid")]
    pub pid: u64,
    #[serde(rename = "ip")]
    pub ip: String,
    #[serde(rename = "external_ip")]
    pub external_ip: String,
    #[serde(rename = "process_name")]
    pub process_name: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "operator_id")]
    pub operator_id: u64,
    #[serde(rename = "active")]
    pub active: bool,
    #[serde(rename = "registered_payload_id")]
    pub registered_payload_id: u64,
    #[serde(rename = "integrity_level")]
    pub integrity_level: u32,
    #[serde(rename = "locked")]
    pub locked: bool,
    #[serde(rename = "locked_operator_id")]
    pub locked_operator_id: u64,
    #[serde(rename = "os")]
    pub os: String,
    #[serde(rename = "architecture")]
    pub architecture: String,
    #[serde(rename = "extra_info")]
    pub extra_info: String,
    #[serde(rename = "sleep_info")]
    pub sleep_info: String,
    #[serde(rename = "payload_type")]
    pub payload_type: String,
    #[serde(rename = "c2_profiles")]
    pub c2_profiles: Vec<PayloadBuildC2Profile>,
    #[serde(rename = "build_parameters")]
    pub build_parameters: Vec<PayloadConfigurationBuildParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTTaskMessagePayloadData {
    #[serde(rename = "os")]
    pub os: String,
    #[serde(rename = "uuid")]
    pub uuid: String,
    #[serde(rename = "payload_type")]
    pub payload_type: String,
    #[serde(rename = "tag")]
    pub tag: String,
    #[serde(rename = "filename")]
    pub filename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtaskGroupName {
    #[serde(rename = "group_name")]
    pub group_name: String,
    #[serde(rename = "task_ids")]
    pub task_ids: Vec<u64>,
}

// Task response structs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTTTaskOPSECPreTaskMessageResponse {
    #[serde(rename = "task_id")]
    pub task_id: u64,
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
    #[serde(rename = "opsec_pre_blocked")]
    pub opsec_pre_blocked: bool,
    #[serde(rename = "opsec_pre_message")]
    pub opsec_pre_message: String,
    #[serde(rename = "opsec_pre_bypassed")]
    pub opsec_pre_bypassed: bool,
    #[serde(rename = "opsec_pre_bypass_role")]
    pub opsec_pre_bypass_role: OpsecRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTTaskOPSECPostTaskMessageResponse {
    #[serde(rename = "task_id")]
    pub task_id: u64,
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
    #[serde(rename = "opsec_post_blocked")]
    pub opsec_post_blocked: bool,
    #[serde(rename = "opsec_post_message")]
    pub opsec_post_message: String,
    #[serde(rename = "opsec_post_bypassed")]
    pub opsec_post_bypassed: bool,
    #[serde(rename = "opsec_post_bypass_role")]
    pub opsec_post_bypass_role: OpsecRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTTaskCreateTaskingMessageResponse {
    #[serde(rename = "task_id")]
    pub task_id: u64,
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
    #[serde(rename = "command_name")]
    pub command_name: String,
    #[serde(rename = "params")]
    pub params: String,
    #[serde(rename = "display_params")]
    pub display_params: String,
    #[serde(rename = "stdout")]
    pub stdout: String,
    #[serde(rename = "stderr")]
    pub stderr: String,
    #[serde(rename = "completed")]
    pub completed: bool,
    #[serde(rename = "status")]
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTTaskCompletionFunctionMessage {
    #[serde(rename = "task")]
    pub task: PTTaskMessageTaskData,
    #[serde(rename = "subtask")]
    pub subtask: Option<PTTaskMessageTaskData>,
    #[serde(rename = "subtask_group")]
    pub subtask_group: Option<SubtaskGroupName>,
    #[serde(rename = "subtask_completion_function")]
    pub subtask_completion_function: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTTaskCompletionFunctionMessageResponse {
    #[serde(rename = "task_id")]
    pub task_id: u64,
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
    #[serde(rename = "params")]
    pub params: String,
    #[serde(rename = "status")]
    pub status: String,
    #[serde(rename = "display_params")]
    pub display_params: String,
    #[serde(rename = "stdout")]
    pub stdout: String,
    #[serde(rename = "stderr")]
    pub stderr: String,
    #[serde(rename = "completed")]
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtTaskProcessResponseMessage {
    #[serde(rename = "task")]
    pub task: PTTaskMessageTaskData,
    #[serde(rename = "response")]
    pub response: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTTaskProcessResponseMessageResponse {
    #[serde(rename = "task_id")]
    pub task_id: u64,
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTOnNewCallbackAllData {
    #[serde(rename = "payload_type")]
    pub payload_type: String,
    #[serde(rename = "callback")]
    pub callback: PTTaskMessageCallbackData,
    #[serde(rename = "build_parameters")]
    pub build_parameters: Vec<PayloadConfigurationBuildParameter>,
    #[serde(rename = "commands")]
    pub commands: Vec<String>,
    #[serde(rename = "c2info")]
    pub c2info: Vec<PayloadBuildC2Profile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTOnNewCallbackResponse {
    #[serde(rename = "agent_callback_id")]
    pub agent_callback_id: String,
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
}

// ---------------------------------------------------------------------------
// Other RPC structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCReSyncMessage {
    #[serde(rename = "payload_type_name")]
    pub payload_type_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCReSyncMessageResponse {
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTCallbacksToCheck {
    #[serde(rename = "agent_callback_id")]
    pub agent_callback_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTCheckIfCallbacksAliveMessage {
    #[serde(rename = "payload_type")]
    pub payload_type: String,
    #[serde(rename = "callbacks")]
    pub callbacks: Vec<PTCallbacksToCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTCallbacksToCheckResponse {
    #[serde(rename = "agent_callback_id")]
    pub agent_callback_id: String,
    #[serde(rename = "active")]
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTCheckIfCallbacksAliveMessageResponse {
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
    #[serde(rename = "callbacks")]
    pub callbacks: Vec<PTCallbacksToCheckResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileBrowserTask {
    #[serde(rename = "host")]
    pub host: String,
    #[serde(rename = "task_id")]
    pub task_id: u64,
    #[serde(rename = "command")]
    pub command: String,
    #[serde(rename = "file")]
    pub file: String,
    #[serde(rename = "parent_path")]
    pub parent_path: String,
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "access_time")]
    pub access_time: String,
    #[serde(rename = "modify_time")]
    pub modify_time: String,
    #[serde(rename = "size")]
    pub size: u64,
    #[serde(rename = "permissions")]
    pub permissions: HashMap<String, Value>,
    #[serde(rename = "extended_file_info")]
    pub extended_file_info: Value,
    #[serde(rename = "files")]
    pub files: Vec<Value>,
    #[serde(rename = "is_file")]
    pub is_file: bool,
    #[serde(rename = "comment")]
    pub comment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCDynamicQueryFunctionMessage {
    #[serde(rename = "command")]
    pub command: String,
    #[serde(rename = "parameter_name")]
    pub parameter_name: String,
    #[serde(rename = "payload_type")]
    pub payload_type: String,
    #[serde(rename = "callback_id")]
    pub callback_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCDynamicQueryFunctionMessageResponse {
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
    #[serde(rename = "choices")]
    pub choices: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCDynamicQueryBuildParameterFunctionMessage {
    #[serde(rename = "parameter_name")]
    pub parameter_name: String,
    #[serde(rename = "payload_type")]
    pub payload_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCDynamicQueryBuildParameterFunctionMessageResponse {
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
    #[serde(rename = "choices")]
    pub choices: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCTypedArrayParseFunctionMessage {
    #[serde(rename = "command")]
    pub command: String,
    #[serde(rename = "parameter_name")]
    pub parameter_name: String,
    #[serde(rename = "payload_type")]
    pub payload_type: String,
    #[serde(rename = "input_array")]
    pub input_array: Vec<String>,
    #[serde(rename = "callback_id")]
    pub callback_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCTypedArrayParseMessageResponse {
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
    #[serde(rename = "typed_array")]
    pub typed_array: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCCommandHelpFunctionMessage {
    #[serde(rename = "command")]
    pub command: String,
    #[serde(rename = "payload_type")]
    pub payload_type: String,
    #[serde(rename = "callback_id")]
    pub callback_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCCommandHelpFunctionMessageResponse {
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "error")]
    pub error: String,
    #[serde(rename = "help")]
    pub help: String,
}
