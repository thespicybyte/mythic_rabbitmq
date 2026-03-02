// Mirrors agent_structs/*.go from MythicMeta/MythicContainer
// JSON tags are kept identical to the Go source so Mythic can deserialize them.
// Structs derived from https://github.com/MythicMeta/MythicContainer/blob/main/agent_structs/structs_payload_sync.go

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

mod timestamp_de {
    use serde::{
        Deserializer,
        de::{self, Visitor},
    };
    use std::fmt;

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
        struct Ts;
        impl<'de> Visitor<'de> for Ts {
            type Value = f64;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "a float or a timestamp string")
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<f64, E> {
                Ok(v)
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<f64, E> {
                Ok(v as f64)
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<f64, E> {
                Ok(v as f64)
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<f64, E> {
                Ok(v.parse::<f64>().unwrap_or(0.0))
            }
        }
        d.deserialize_any(Ts)
    }
}

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

impl Default for AgentType {
    fn default() -> Self {
        AgentType::Agent
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageFormat {
    Json,
    Xml,
}

impl Default for MessageFormat {
    fn default() -> Self {
        MessageFormat::Json
    }
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

impl Default for HideConditionOperand {
    fn default() -> Self {
        HideConditionOperand::Eq
    }
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
    #[serde(rename = "payload_type", default)]
    pub payload_type: PayloadType,
    #[serde(rename = "commands", default)]
    pub commands: Vec<Command>,
    #[serde(rename = "container_version", default)]
    pub container_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadTypeSyncMessageResponse {
    #[serde(rename = "success", default)]
    pub success: bool,
    #[serde(rename = "error", default)]
    pub error: String,
}

/// Payload type definition (serializable fields only; function pointers excluded).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PayloadType {
    #[serde(rename = "name", default)]
    pub name: String,
    #[serde(rename = "file_extension", default)]
    pub file_extension: String,
    #[serde(rename = "author", default)]
    pub author: String,
    #[serde(rename = "semver", default)]
    pub version: String,
    #[serde(rename = "supported_os", default)]
    pub supported_os: Vec<String>,
    #[serde(rename = "wrapper", default)]
    pub wrapper: bool,
    #[serde(rename = "wrapped_payloads", default)]
    pub wrapped_payloads: Vec<String>,
    #[serde(rename = "description", default)]
    pub description: String,
    #[serde(rename = "supports_dynamic_loading", default)]
    pub supports_dynamic_loading: bool,
    #[serde(rename = "build_parameters", default)]
    pub build_parameters: Vec<BuildParameter>,
    #[serde(rename = "build_steps", default)]
    pub build_steps: Vec<BuildStep>,
    #[serde(rename = "message_format", default)]
    pub message_format: MessageFormat,
    #[serde(rename = "agent_type", default)]
    pub agent_type: AgentType,
    #[serde(rename = "supported_c2_profiles", default)]
    pub supported_c2_profiles: Vec<String>,
    #[serde(rename = "mythic_encrypts", default)]
    pub mythic_encrypts: bool,
    #[serde(rename = "translation_container_name", default)]
    pub translation_container_name: String,
    #[serde(rename = "agent_icon", default)]
    pub agent_icon: Option<Vec<u8>>,
    #[serde(rename = "dark_mode_agent_icon", default)]
    pub dark_mode_agent_icon: Option<Vec<u8>>,
    #[serde(rename = "custom_rpc_functions", default)]
    pub custom_rpc_functions: Vec<String>,
}

impl PayloadType {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = author.into();
        self
    }

    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    pub fn file_extension(mut self, file_extension: impl Into<String>) -> Self {
        self.file_extension = file_extension.into();
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn supported_os(mut self, oses: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.supported_os = oses.into_iter().map(Into::into).collect();
        self
    }

    pub fn build_steps(mut self, steps: impl IntoIterator<Item = BuildStep>) -> Self {
        self.build_steps = steps.into_iter().collect();
        self
    }

    pub fn build_parameters(mut self, params: impl IntoIterator<Item = BuildParameter>) -> Self {
        self.build_parameters = params.into_iter().collect();
        self
    }

    pub fn message_format(mut self, message_format: MessageFormat) -> Self {
        self.message_format = message_format;
        self
    }

    pub fn agent_type(mut self, agent_type: AgentType) -> Self {
        self.agent_type = agent_type;
        self
    }

    pub fn agent_icon(mut self, icon: Vec<u8>) -> Self {
        self.agent_icon = Some(icon);
        self
    }

    pub fn dark_mode_agent_icon(mut self, icon: Vec<u8>) -> Self {
        self.dark_mode_agent_icon = Some(icon);
        self
    }

    pub fn wrapper(mut self, wrapper: bool) -> Self {
        self.wrapper = wrapper;
        self
    }

    pub fn wrapped_payloads(
        mut self,
        payloads: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.wrapped_payloads = payloads.into_iter().map(Into::into).collect();
        self
    }

    pub fn supports_dynamic_loading(mut self, supports: bool) -> Self {
        self.supports_dynamic_loading = supports;
        self
    }

    pub fn mythic_encrypts(mut self, mythic_encrypts: bool) -> Self {
        self.mythic_encrypts = mythic_encrypts;
        self
    }

    pub fn translation_container_name(mut self, name: impl Into<String>) -> Self {
        self.translation_container_name = name.into();
        self
    }

    pub fn supported_c2_profiles(
        mut self,
        profiles: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.supported_c2_profiles = profiles.into_iter().map(Into::into).collect();
        self
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    #[serde(rename = "name", default)]
    pub name: String,
    #[serde(rename = "needs_admin_permissions", default)]
    pub needs_admin_permissions: bool,
    #[serde(rename = "help_cmd", default)]
    pub help_cmd: String,
    #[serde(rename = "description", default)]
    pub description: String,
    #[serde(rename = "version", default)]
    pub version: u32,
    #[serde(rename = "supported_ui_features", default)]
    pub supported_ui_features: Vec<String>,
    #[serde(rename = "author", default)]
    pub author: String,
    #[serde(rename = "mitre_attack_techniques", default)]
    pub mitre_attack_techniques: Vec<String>,
    #[serde(rename = "attributes", default)]
    pub attributes: CommandAttribute,
    #[serde(rename = "script_only", default)]
    pub script_only: bool,
    #[serde(rename = "parameters", default)]
    pub parameters: Vec<CommandParameter>,
    #[serde(rename = "browser_script", default)]
    pub browser_script: Option<BrowserScript>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandParameter {
    #[serde(rename = "name", default)]
    pub name: String,
    #[serde(rename = "display_name", default)]
    pub display_name: String,
    #[serde(rename = "cli_name", default)]
    pub cli_name: String,
    #[serde(rename = "parameter_type", default)]
    pub parameter_type: String,
    #[serde(rename = "description", default)]
    pub description: String,
    #[serde(rename = "choices", default)]
    pub choices: Vec<String>,
    #[serde(rename = "default_value", default)]
    pub default_value: Value,
    #[serde(rename = "required", default)]
    pub required: bool,
    #[serde(rename = "verifier_regex", default)]
    pub verifier_regex: String,
    #[serde(rename = "supported_agents", default)]
    pub supported_agents: Vec<String>,
    #[serde(rename = "supported_agent_build_parameters", default)]
    pub supported_agent_build_parameters: HashMap<String, String>,
    #[serde(rename = "choices_are_all_commands", default)]
    pub choices_are_all_commands: bool,
    #[serde(rename = "choices_are_loaded_commands", default)]
    pub choices_are_loaded_commands: bool,
    #[serde(rename = "filter_command_choices_by_supported_os", default)]
    pub filter_command_choices_by_supported_os: bool,
    #[serde(rename = "dynamic_query_function", default)]
    pub dynamic_query_function: Option<String>,
    #[serde(rename = "typedarray_parse_function", default)]
    pub typedarray_parse_function: Option<String>,
    #[serde(rename = "parameter_group_info", default)]
    pub parameter_group_info: Vec<ParameterGroupInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterGroupInfo {
    #[serde(rename = "group_name", default)]
    pub group_name: String,
    #[serde(rename = "parameter_is_required", default)]
    pub parameter_is_required: bool,
    #[serde(rename = "ui_position", default)]
    pub ui_position: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommandAttribute {
    #[serde(rename = "filter_by_build_parameter", default)]
    pub filter_by_build_parameter: HashMap<String, String>,
    #[serde(rename = "supported_os", default)]
    pub supported_os: Vec<String>,
    #[serde(rename = "builtin", default)]
    pub builtin: bool,
    #[serde(rename = "suggested_command", default)]
    pub suggested_command: bool,
    #[serde(rename = "load_only", default)]
    pub load_only: bool,
    #[serde(rename = "script_only", default)]
    pub script_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserScript {
    #[serde(rename = "script", default)]
    pub script: String,
    #[serde(rename = "author", default)]
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildParameter {
    #[serde(rename = "name", default)]
    pub name: String,
    #[serde(rename = "description", default)]
    pub description: String,
    #[serde(rename = "required", default)]
    pub required: bool,
    #[serde(rename = "verifier_regex", default)]
    pub verifier_regex: String,
    #[serde(rename = "default_value", default)]
    pub default_value: Value,
    #[serde(rename = "parameter_type", default)]
    pub parameter_type: String,
    #[serde(rename = "choices", default)]
    pub choices: Vec<String>,
    #[serde(rename = "randomize", default)]
    pub randomize: bool,
    #[serde(rename = "format_string", default)]
    pub format_string: String,
    #[serde(rename = "crypto_type", default)]
    pub is_crypto_type: bool,
    #[serde(rename = "hide_conditions", default)]
    pub hide_conditions: Vec<BuildParameterHideCondition>,
    #[serde(rename = "dictionary_choices", default)]
    pub dictionary_choices: Vec<BuildParameterDictionary>,
}

impl BuildParameter {
    /// Create a new parameter with the two required fields.
    /// All other fields default to empty/false/null.
    pub fn new(name: impl Into<String>, parameter_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parameter_type: parameter_type.into(),
            ..Default::default()
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn default_value(mut self, value: impl Into<Value>) -> Self {
        self.default_value = value.into();
        self
    }

    pub fn choices(mut self, choices: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.choices = choices.into_iter().map(Into::into).collect();
        self
    }

    pub fn verifier_regex(mut self, regex: impl Into<String>) -> Self {
        self.verifier_regex = regex.into();
        self
    }

    pub fn randomize(mut self, randomize: bool) -> Self {
        self.randomize = randomize;
        self
    }

    pub fn format_string(mut self, format_string: impl Into<String>) -> Self {
        self.format_string = format_string.into();
        self
    }

    pub fn crypto_type(mut self, is_crypto_type: bool) -> Self {
        self.is_crypto_type = is_crypto_type;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildParameterHideCondition {
    #[serde(rename = "parameter_name", default)]
    pub parameter_name: String,
    #[serde(rename = "operand", default)]
    pub operand: HideConditionOperand,
    #[serde(rename = "value", default)]
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildParameterDictionary {
    #[serde(rename = "name", default)]
    pub name: String,
    #[serde(rename = "default_value", default)]
    pub default_value: String,
    #[serde(rename = "required", default)]
    pub required: bool,
    #[serde(rename = "crypto_type", default)]
    pub is_crypto_type: bool,
    #[serde(rename = "description", default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildStep {
    #[serde(rename = "step_name", default)]
    pub step_name: String,
    #[serde(rename = "step_description", default)]
    pub step_description: String,
}

impl BuildStep {
    pub fn new(step_name: impl Into<String>, step_description: impl Into<String>) -> Self {
        Self {
            step_name: step_name.into(),
            step_description: step_description.into(),
        }
    }
}

/// C2ParameterDictionary for the PT side (mirrors agent_structs version).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentC2ParameterDictionary {
    #[serde(rename = "name", default)]
    pub name: String,
    #[serde(rename = "default_value", default)]
    pub default_value: String,
    #[serde(rename = "required", default)]
    pub required: bool,
    #[serde(rename = "crypto_type", default)]
    pub is_crypto_type: bool,
    #[serde(rename = "description", default)]
    pub description: String,
}

/// C2ParameterDeviation describes a mismatch between what this payload type
/// expects from a C2 profile and what the C2 profile actually provides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2ParameterDeviation {
    #[serde(rename = "c2_profile_name", default)]
    pub c2_profile_name: String,
    #[serde(rename = "parameter_name", default)]
    pub parameter_name: String,
    #[serde(rename = "description", default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadBuildC2ProfileMessage {
    #[serde(rename = "name", default)]
    pub name: String,
    #[serde(rename = "parameters", default)]
    pub parameters: HashMap<String, Value>,
    #[serde(rename = "crypto_params", default)]
    pub crypto_params: Vec<AgentC2ParameterDictionary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadBuildC2ProfileMessageResponse {
    #[serde(rename = "success", default)]
    pub success: bool,
    #[serde(rename = "error", default)]
    pub error: String,
}

/// Payload configuration included in build messages from Mythic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadConfiguration {
    #[serde(rename = "payload_type", default)]
    pub payload_type: String,
    #[serde(rename = "description", default)]
    pub description: String,
    #[serde(rename = "tag", default)]
    pub tag: String,
    #[serde(rename = "uuid", default)]
    pub uuid: String,
    #[serde(rename = "os", default)]
    pub os: String,
    #[serde(rename = "c2_profiles", default)]
    pub c2_profiles: Vec<PayloadConfigurationC2Profile>,
    #[serde(rename = "build_parameters", default)]
    pub build_parameters: Vec<PayloadConfigurationBuildParameter>,
    #[serde(rename = "commands", default)]
    pub commands: Vec<String>,
    #[serde(rename = "wrapped_payload", default)]
    pub wrapped_payload: String,
    #[serde(rename = "selected_os", default)]
    pub selected_os: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadConfigurationC2Profile {
    #[serde(rename = "name", default)]
    pub name: String,
    #[serde(rename = "parameters", default)]
    pub parameters: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadConfigurationBuildParameter {
    #[serde(rename = "name", default)]
    pub name: String,
    #[serde(rename = "value", default)]
    pub value: Value,
}

/// Message for other-service RPC calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCOtherServiceRPCMessage {
    #[serde(rename = "service_name", default)]
    pub service_name: String,
    #[serde(rename = "service_rpc_function", default)]
    pub service_rpc_function: String,
    #[serde(rename = "task_id", default)]
    pub task_id: u64,
    #[serde(rename = "message", default)]
    pub message: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCOtherServiceRPCMessageResponse {
    #[serde(rename = "success", default)]
    pub success: bool,
    #[serde(rename = "error", default)]
    pub error: String,
    #[serde(rename = "result", default)]
    pub result: HashMap<String, Value>,
}

// ---------------------------------------------------------------------------
// Mythic RPC structs (container → Mythic)
// ---------------------------------------------------------------------------

/// Sent to Mythic to update the UI progress indicator during a build.
/// Mirrors `MythicRPCPayloadUpdateBuildStepMessage` in Go.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MythicRPCPayloadUpdateBuildStepMessage {
    /// UUID of the payload being built (from [`PayloadBuildMessage::uuid`]).
    #[serde(rename = "payload_uuid")]
    pub payload_uuid: String,
    /// Must match a `step_name` declared in your `PayloadType::build_steps`.
    #[serde(rename = "step_name")]
    pub step_name: String,
    /// Human-readable progress message shown in the Mythic UI.
    #[serde(rename = "step_stdout")]
    pub step_stdout: String,
    /// Error detail for this step; leave empty on success.
    #[serde(rename = "step_stderr")]
    pub step_stderr: String,
    /// Whether this step completed successfully.
    #[serde(rename = "step_success")]
    pub step_success: bool,
    /// Mark this step as skipped (not run).
    #[serde(rename = "step_skip")]
    pub step_skip: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MythicRPCPayloadUpdateBuildStepMessageResponse {
    #[serde(rename = "success", default)]
    pub success: bool,
    #[serde(rename = "error", default)]
    pub error: String,
}

// ---------------------------------------------------------------------------
// Build structs (from structs_payload_build.go)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadBuildMessage {
    #[serde(rename = "payload_type", default)]
    pub payload_type: String,
    #[serde(rename = "filename", default)]
    pub filename: String,
    #[serde(rename = "tag", default)]
    pub tag: String,
    #[serde(rename = "selected_os", default)]
    pub selected_os: String,
    #[serde(rename = "commands", default)]
    pub commands: Vec<String>,
    #[serde(rename = "build_parameters", default)]
    pub build_parameters: BuildParameters,
    #[serde(rename = "c2_profiles", default)]
    pub c2_profiles: Vec<PayloadBuildC2Profile>,
    #[serde(rename = "wrapped_payload", default)]
    pub wrapped_payload: String,
    #[serde(rename = "wrapped_payload_uuid", default)]
    pub wrapped_payload_uuid: String,
    #[serde(rename = "uuid", default)]
    pub uuid: String,
    #[serde(rename = "all_c2_info", default)]
    pub all_c2_info: Vec<PayloadBuildC2ProfileMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadBuildC2Profile {
    #[serde(rename = "name", default)]
    pub name: String,
    #[serde(rename = "parameters", default)]
    pub parameters: HashMap<String, Value>,
    #[serde(rename = "crypto_params", default)]
    pub crypto_params: Vec<CryptoArg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoArg {
    #[serde(rename = "enc_key", default)]
    pub enc_key: Option<String>,
    #[serde(rename = "dec_key", default)]
    pub dec_key: Option<String>,
    #[serde(rename = "type", default)]
    pub crypto_type: String,
    #[serde(rename = "value", default)]
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BuildParameters(pub HashMap<String, Value>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayloadBuildResponse {
    #[serde(rename = "uuid", default)]
    pub uuid: String,
    #[serde(rename = "success", default)]
    pub success: bool,
    #[serde(rename = "error", default)]
    pub error: String,
    #[serde(rename = "payload", default)]
    pub payload: Option<Vec<u8>>,
    #[serde(rename = "status", default)]
    pub status: String,
    #[serde(rename = "build_message", default)]
    pub build_message: String,
    #[serde(rename = "build_stderr", default)]
    pub build_stderr: String,
    #[serde(rename = "build_stdout", default)]
    pub build_stdout: String,
    #[serde(rename = "updated_command_list", default)]
    pub updated_command_list: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Task message structs (from structs_task_messages.go)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTTaskMessageAllData {
    #[serde(rename = "task", default)]
    pub task: PTTaskMessageTaskData,
    #[serde(rename = "callback", default)]
    pub callback: PTTaskMessageCallbackData,
    #[serde(rename = "payload", default)]
    pub payload: PTTaskMessagePayloadData,
    #[serde(rename = "build_parameters", default)]
    pub build_parameters: Vec<PayloadConfigurationBuildParameter>,
    #[serde(rename = "commands", default)]
    pub commands: Vec<String>,
    #[serde(rename = "c2info", default)]
    pub c2info: Vec<PayloadBuildC2Profile>,
    #[serde(rename = "subtasks", default)]
    pub subtasks: Vec<PTTaskMessageTaskData>,
    #[serde(rename = "subtask_groups", default)]
    pub subtask_groups: Vec<SubtaskGroupName>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PTTaskMessageTaskData {
    #[serde(rename = "id", default)]
    pub id: u64,
    #[serde(rename = "display_id", default)]
    pub display_id: u64,
    #[serde(rename = "agent_task_id", default)]
    pub agent_task_id: String,
    #[serde(rename = "command_name", default)]
    pub command_name: String,
    #[serde(rename = "params", default)]
    pub params: String,
    #[serde(
        rename = "timestamp",
        deserialize_with = "timestamp_de::deserialize",
        default
    )]
    pub timestamp: f64,
    #[serde(rename = "callback_id", default)]
    pub callback_id: u64,
    #[serde(rename = "status", default)]
    pub status: String,
    #[serde(rename = "original_params", default)]
    pub original_params: String,
    #[serde(rename = "display_params", default)]
    pub display_params: String,
    #[serde(rename = "comment", default)]
    pub comment: String,
    #[serde(rename = "stdout", default)]
    pub stdout: String,
    #[serde(rename = "stderr", default)]
    pub stderr: String,
    #[serde(rename = "completed", default)]
    pub completed: bool,
    #[serde(rename = "operator_username", default)]
    pub operator_username: String,
    #[serde(rename = "opsec_pre_bypassed", default)]
    pub opsec_pre_bypassed: bool,
    #[serde(rename = "opsec_post_bypassed", default)]
    pub opsec_post_bypassed: bool,
    #[serde(rename = "opsec_pre_bypass_role", default)]
    pub opsec_pre_bypass_role: OpsecRole,
    #[serde(rename = "opsec_post_bypass_role", default)]
    pub opsec_post_bypass_role: OpsecRole,
    #[serde(rename = "tags", default)]
    pub tags: Vec<String>,
    #[serde(rename = "token", default)]
    pub token: Option<Value>,
    #[serde(rename = "subtask_callback_function", default)]
    pub subtask_callback_function: String,
    #[serde(rename = "group_callback_function", default)]
    pub group_callback_function: String,
    #[serde(rename = "completed_callback_function", default)]
    pub completed_callback_function: String,
    #[serde(rename = "subtask_group_name", default)]
    pub subtask_group_name: String,
    #[serde(rename = "tasking_location", default)]
    pub tasking_location: String,
    #[serde(rename = "parameter_group_name", default)]
    pub parameter_group_name: String,
    #[serde(rename = "args", default)]
    pub args: PTTaskMessageArgsData,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PTTaskMessageArgsData {
    #[serde(rename = "command_line", default)]
    pub command_line: String,
    #[serde(rename = "tasking_location", default)]
    pub tasking_location: String,
    #[serde(rename = "parameter_group_name", default)]
    pub parameter_group_name: String,
    #[serde(flatten)]
    pub raw_params: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PTTaskMessageCallbackData {
    #[serde(rename = "id", default)]
    pub id: u64,
    #[serde(rename = "display_id", default)]
    pub display_id: u64,
    #[serde(rename = "agent_callback_id", default)]
    pub agent_callback_id: String,
    #[serde(
        rename = "init_callback",
        default,
        deserialize_with = "timestamp_de::deserialize"
    )]
    pub init_callback: f64,
    #[serde(
        rename = "last_checkin",
        default,
        deserialize_with = "timestamp_de::deserialize"
    )]
    pub last_checkin: f64,
    #[serde(rename = "user", default)]
    pub user: String,
    #[serde(rename = "host", default)]
    pub host: String,
    #[serde(rename = "pid", default)]
    pub pid: u64,
    #[serde(rename = "ip", default)]
    pub ip: String,
    #[serde(rename = "external_ip", default)]
    pub external_ip: String,
    #[serde(rename = "process_name", default)]
    pub process_name: String,
    #[serde(rename = "description", default)]
    pub description: String,
    #[serde(rename = "operator_id", default)]
    pub operator_id: u64,
    #[serde(rename = "active", default)]
    pub active: bool,
    #[serde(rename = "registered_payload_id", default)]
    pub registered_payload_id: u64,
    #[serde(rename = "integrity_level", default)]
    pub integrity_level: u32,
    #[serde(rename = "locked", default)]
    pub locked: bool,
    #[serde(rename = "locked_operator_id", default)]
    pub locked_operator_id: u64,
    #[serde(rename = "os", default)]
    pub os: String,
    #[serde(rename = "architecture", default)]
    pub architecture: String,
    #[serde(rename = "extra_info", default)]
    pub extra_info: String,
    #[serde(rename = "sleep_info", default)]
    pub sleep_info: String,
    #[serde(rename = "payload_type", default)]
    pub payload_type: String,
    #[serde(rename = "c2_profiles", default)]
    pub c2_profiles: Vec<PayloadBuildC2Profile>,
    #[serde(rename = "build_parameters", default)]
    pub build_parameters: Vec<PayloadConfigurationBuildParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PTTaskMessagePayloadData {
    #[serde(rename = "os", default)]
    pub os: String,
    #[serde(rename = "uuid", default)]
    pub uuid: String,
    #[serde(rename = "payload_type", default)]
    pub payload_type: String,
    #[serde(rename = "tag", default)]
    pub tag: String,
    #[serde(rename = "filename", default)]
    pub filename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtaskGroupName {
    #[serde(rename = "group_name", default)]
    pub group_name: String,
    #[serde(rename = "task_ids", default)]
    pub task_ids: Vec<u64>,
}

// Task response structs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTTTaskOPSECPreTaskMessageResponse {
    #[serde(rename = "task_id", default)]
    pub task_id: u64,
    #[serde(rename = "success", default)]
    pub success: bool,
    #[serde(rename = "error", default)]
    pub error: String,
    #[serde(rename = "opsec_pre_blocked", default)]
    pub opsec_pre_blocked: bool,
    #[serde(rename = "opsec_pre_message", default)]
    pub opsec_pre_message: String,
    #[serde(rename = "opsec_pre_bypassed", default)]
    pub opsec_pre_bypassed: bool,
    #[serde(rename = "opsec_pre_bypass_role", default)]
    pub opsec_pre_bypass_role: OpsecRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTTaskOPSECPostTaskMessageResponse {
    #[serde(rename = "task_id", default)]
    pub task_id: u64,
    #[serde(rename = "success", default)]
    pub success: bool,
    #[serde(rename = "error", default)]
    pub error: String,
    #[serde(rename = "opsec_post_blocked", default)]
    pub opsec_post_blocked: bool,
    #[serde(rename = "opsec_post_message", default)]
    pub opsec_post_message: String,
    #[serde(rename = "opsec_post_bypassed", default)]
    pub opsec_post_bypassed: bool,
    #[serde(rename = "opsec_post_bypass_role", default)]
    pub opsec_post_bypass_role: OpsecRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTTaskCreateTaskingMessageResponse {
    #[serde(rename = "task_id", default)]
    pub task_id: u64,
    #[serde(rename = "success", default)]
    pub success: bool,
    #[serde(rename = "error", default)]
    pub error: String,
    #[serde(rename = "command_name", default)]
    pub command_name: String,
    #[serde(rename = "params", default)]
    pub params: String,
    #[serde(rename = "display_params", default)]
    pub display_params: String,
    #[serde(rename = "stdout", default)]
    pub stdout: String,
    #[serde(rename = "stderr", default)]
    pub stderr: String,
    #[serde(rename = "completed", default)]
    pub completed: bool,
    #[serde(rename = "status", default)]
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTTaskCompletionFunctionMessage {
    #[serde(rename = "task")]
    pub task: PTTaskMessageTaskData,
    #[serde(rename = "subtask", default)]
    pub subtask: Option<PTTaskMessageTaskData>,
    #[serde(rename = "subtask_group", default)]
    pub subtask_group: Option<SubtaskGroupName>,
    #[serde(rename = "subtask_completion_function", default)]
    pub subtask_completion_function: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTTaskCompletionFunctionMessageResponse {
    #[serde(rename = "task_id", default)]
    pub task_id: u64,
    #[serde(rename = "success", default)]
    pub success: bool,
    #[serde(rename = "error", default)]
    pub error: String,
    #[serde(rename = "params", default)]
    pub params: String,
    #[serde(rename = "status", default)]
    pub status: String,
    #[serde(rename = "display_params", default)]
    pub display_params: String,
    #[serde(rename = "stdout", default)]
    pub stdout: String,
    #[serde(rename = "stderr", default)]
    pub stderr: String,
    #[serde(rename = "completed", default)]
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PtTaskProcessResponseMessage {
    #[serde(rename = "task")]
    pub task: PTTaskMessageTaskData,
    #[serde(rename = "response", default)]
    pub response: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTTaskProcessResponseMessageResponse {
    #[serde(rename = "task_id", default)]
    pub task_id: u64,
    #[serde(rename = "success", default)]
    pub success: bool,
    #[serde(rename = "error", default)]
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTOnNewCallbackAllData {
    #[serde(rename = "payload_type", default)]
    pub payload_type: String,
    #[serde(rename = "callback")]
    pub callback: PTTaskMessageCallbackData,
    #[serde(rename = "build_parameters", default)]
    pub build_parameters: Vec<PayloadConfigurationBuildParameter>,
    #[serde(rename = "commands", default)]
    pub commands: Vec<String>,
    #[serde(rename = "c2info", default)]
    pub c2info: Vec<PayloadBuildC2Profile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTOnNewCallbackResponse {
    #[serde(rename = "agent_callback_id", default)]
    pub agent_callback_id: String,
    #[serde(rename = "success", default)]
    pub success: bool,
    #[serde(rename = "error", default)]
    pub error: String,
}

// ---------------------------------------------------------------------------
// Other RPC structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCReSyncMessage {
    #[serde(rename = "payload_type_name", default)]
    pub payload_type_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCReSyncMessageResponse {
    #[serde(rename = "success", default)]
    pub success: bool,
    #[serde(rename = "error", default)]
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTCallbacksToCheck {
    #[serde(rename = "agent_callback_id", default)]
    pub agent_callback_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTCheckIfCallbacksAliveMessage {
    #[serde(rename = "payload_type", default)]
    pub payload_type: String,
    #[serde(rename = "callbacks", default)]
    pub callbacks: Vec<PTCallbacksToCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTCallbacksToCheckResponse {
    #[serde(rename = "agent_callback_id", default)]
    pub agent_callback_id: String,
    #[serde(rename = "active", default)]
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTCheckIfCallbacksAliveMessageResponse {
    #[serde(rename = "success", default)]
    pub success: bool,
    #[serde(rename = "error", default)]
    pub error: String,
    #[serde(rename = "callbacks", default)]
    pub callbacks: Vec<PTCallbacksToCheckResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileBrowserTask {
    #[serde(rename = "host", default)]
    pub host: String,
    #[serde(rename = "task_id", default)]
    pub task_id: u64,
    #[serde(rename = "command", default)]
    pub command: String,
    #[serde(rename = "file", default)]
    pub file: String,
    #[serde(rename = "parent_path", default)]
    pub parent_path: String,
    #[serde(rename = "success", default)]
    pub success: bool,
    #[serde(rename = "access_time", default)]
    pub access_time: String,
    #[serde(rename = "modify_time", default)]
    pub modify_time: String,
    #[serde(rename = "size", default)]
    pub size: u64,
    #[serde(rename = "permissions", default)]
    pub permissions: HashMap<String, Value>,
    #[serde(rename = "extended_file_info", default)]
    pub extended_file_info: Value,
    #[serde(rename = "files", default)]
    pub files: Vec<Value>,
    #[serde(rename = "is_file", default)]
    pub is_file: bool,
    #[serde(rename = "comment", default)]
    pub comment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCDynamicQueryFunctionMessage {
    #[serde(rename = "command", default)]
    pub command: String,
    #[serde(rename = "parameter_name", default)]
    pub parameter_name: String,
    #[serde(rename = "payload_type", default)]
    pub payload_type: String,
    #[serde(rename = "callback_id", default)]
    pub callback_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCDynamicQueryFunctionMessageResponse {
    #[serde(rename = "success", default)]
    pub success: bool,
    #[serde(rename = "error", default)]
    pub error: String,
    #[serde(rename = "choices", default)]
    pub choices: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCDynamicQueryBuildParameterFunctionMessage {
    #[serde(rename = "parameter_name", default)]
    pub parameter_name: String,
    #[serde(rename = "payload_type", default)]
    pub payload_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCDynamicQueryBuildParameterFunctionMessageResponse {
    #[serde(rename = "success", default)]
    pub success: bool,
    #[serde(rename = "error", default)]
    pub error: String,
    #[serde(rename = "choices", default)]
    pub choices: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCTypedArrayParseFunctionMessage {
    #[serde(rename = "command", default)]
    pub command: String,
    #[serde(rename = "parameter_name", default)]
    pub parameter_name: String,
    #[serde(rename = "payload_type", default)]
    pub payload_type: String,
    #[serde(rename = "input_array", default)]
    pub input_array: Vec<String>,
    #[serde(rename = "callback_id", default)]
    pub callback_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCTypedArrayParseMessageResponse {
    #[serde(rename = "success", default)]
    pub success: bool,
    #[serde(rename = "error", default)]
    pub error: String,
    #[serde(rename = "typed_array", default)]
    pub typed_array: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCCommandHelpFunctionMessage {
    #[serde(rename = "command", default)]
    pub command: String,
    #[serde(rename = "payload_type", default)]
    pub payload_type: String,
    #[serde(rename = "callback_id", default)]
    pub callback_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PTRPCCommandHelpFunctionMessageResponse {
    #[serde(rename = "success", default)]
    pub success: bool,
    #[serde(rename = "error", default)]
    pub error: String,
    #[serde(rename = "help", default)]
    pub help: String,
}
