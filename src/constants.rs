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
