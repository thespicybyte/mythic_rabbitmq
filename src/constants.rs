// Mirrors constants.go from MythicMeta/MythicContainer

pub const MYTHIC_EXCHANGE: &str = "mythic_exchange";
pub const CONTAINER_VERSION: &str = "v1.4.3";

pub const RETRY_CONNECT_DELAY_SECS: u64 = 5;
pub const RPC_TIMEOUT_SECS: u64 = 30;

// C2 sync routing key - sent once on startup to register the profile
pub const C2_SYNC_ROUTING_KEY: &str = "c2_sync";

// C2 RPC base routing keys - prefixed with "{c2_name}_" at runtime via get_routing_key()
pub const C2_RPC_CONFIG_CHECK_ROUTING_KEY: &str = "c2_rpc_config_check";
pub const C2_RPC_OPSEC_CHECK_ROUTING_KEY: &str = "c2_rpc_opsec_check";
pub const C2_RPC_GET_IOC_ROUTING_KEY: &str = "c2_rpc_get_ioc";
pub const C2_RPC_SAMPLE_MESSAGE_ROUTING_KEY: &str = "c2_rpc_sample_message";
pub const C2_RPC_REDIRECTOR_RULES_ROUTING_KEY: &str = "c2_rpc_redirector_rules";
pub const C2_RPC_HOST_FILE_ROUTING_KEY: &str = "c2_rpc_host_file";
pub const C2_RPC_START_SERVER_ROUTING_KEY: &str = "c2_rpc_start_server";
pub const C2_RPC_STOP_SERVER_ROUTING_KEY: &str = "c2_rpc_stop_server";
pub const C2_RPC_GET_SERVER_DEBUG_OUTPUT_ROUTING_KEY: &str = "c2_rpc_get_server_debug_output";
pub const C2_RPC_RESYNC_ROUTING_KEY: &str = "c2_rpc_resync";

// Mythic RPC - called by the container TO Mythic (not namespaced with c2 name)
pub const MYTHIC_RPC_C2_UPDATE_STATUS: &str = "mythic_rpc_c2_update_status";

/// Construct the namespaced routing key for a given C2 profile and base key.
/// e.g. get_routing_key("reverse_tcp", C2_RPC_CONFIG_CHECK_ROUTING_KEY)
///   -> "reverse_tcp_c2_rpc_config_check"
pub fn get_routing_key(c2_name: &str, base_key: &str) -> String {
    format!("{}_{}", c2_name, base_key)
}

// ---------------------------------------------------------------------------
// PT (Payload Type / Agent) routing keys
// ---------------------------------------------------------------------------

// PT sync
pub const PT_SYNC_ROUTING_KEY: &str = "pt_sync";
pub const PT_RPC_RESYNC_ROUTING_KEY: &str = "pt_rpc_resync";

// PT inbound queues (prefixed with {pt_name}_ at runtime)
pub const PT_PAYLOAD_BUILD_ROUTING_KEY: &str = "payload_build";
pub const PT_ON_NEW_CALLBACK_ROUTING_KEY: &str = "pt_on_new_callback";
pub const PT_CHECK_IF_CALLBACKS_ALIVE_ROUTING_KEY: &str = "pt_check_if_callbacks_alive";
pub const PT_TASK_CREATE_TASKING_ROUTING_KEY: &str = "pt_task_create_tasking";
pub const PT_TASK_OPSEC_PRE_ROUTING_KEY: &str = "pt_task_opsec_pre_check";
pub const PT_TASK_OPSEC_POST_ROUTING_KEY: &str = "pt_task_opsec_post_check";
pub const PT_TASK_PROCESS_RESPONSE_ROUTING_KEY: &str = "pt_task_process_response";
pub const PT_TASK_COMPLETION_FUNCTION_ROUTING_KEY: &str = "pt_task_completion_function";
pub const PT_RPC_COMMAND_DYNAMIC_QUERY_ROUTING_KEY: &str = "pt_command_dynamic_query_function";
pub const PT_RPC_BUILD_PARAM_DYNAMIC_QUERY_ROUTING_KEY: &str =
    "pt_build_parameter_dynamic_query_function";
pub const PT_RPC_COMMAND_TYPEDARRAY_PARSE_ROUTING_KEY: &str = "pt_command_typedarray_parse";
pub const PT_RPC_COMMAND_HELP_ROUTING_KEY: &str = "pt_command_help_function";

// PT outbound (publish to Mythic, not namespaced with pt_name)
pub const PT_BUILD_RESPONSE_ROUTING_KEY: &str = "pt_build_response";
pub const PT_TASK_CREATE_TASKING_RESPONSE_ROUTING_KEY: &str = "pt_task_create_tasking_response";
pub const PT_TASK_OPSEC_PRE_RESPONSE_ROUTING_KEY: &str = "pt_task_opsec_pre_check_response";
pub const PT_TASK_OPSEC_POST_RESPONSE_ROUTING_KEY: &str = "pt_task_opsec_post_check_response";
pub const PT_TASK_PROCESS_RESPONSE_RESPONSE_ROUTING_KEY: &str =
    "pt_task_process_response_response";
pub const PT_TASK_COMPLETION_RESPONSE_ROUTING_KEY: &str = "pt_task_completion_function_response";
pub const PT_ON_NEW_CALLBACK_RESPONSE_ROUTING_KEY: &str = "pt_on_new_callback_response";
