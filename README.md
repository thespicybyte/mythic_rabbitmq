# mythic-rabbitmq

A Rust port of the RabbitMQ layer from [MythicMeta/MythicContainer](https://github.com/MythicMeta/MythicContainer).

Handles C2 profile registration and RPC dispatch for [Mythic](https://github.com/its-a-feature/Mythic) C2 containers. Replaces the gRPC transport used in older Mythic versions — modern Mythic communicates with containers entirely over AMQP 0-9-1 (RabbitMQ).

## What it does

On startup, `start_and_run_forever`:

1. Connects to RabbitMQ (retries forever)
2. Sends a `c2_sync` RPC to register the profile definition and parameters with Mythic
3. Launches the server binary automatically
4. Sends `mythic_rpc_c2_update_status` to flip the container online in the Mythic UI
5. Spawns background tasks listening on all C2 RPC queues
6. Blocks until Ctrl+C

When Mythic sends a `start_server` or `stop_server` RPC, the container launches or kills the server binary via `tokio::process::Command`. The process handle is shared between both listeners via `Arc<Mutex<Option<Child>>>` so stop can kill the exact process that was started.

## Usage

Add to `Cargo.toml`:

```toml
[dependencies]
mythic-rabbitmq = { path = "../mythic_rabbitmq_rs" }
tokio = { version = "1.0", features = ["full"] }
tracing = "0.1"
tracing-subscriber = "0.3"
```

Minimal example:

```rust
use mythic_rabbitmq::{MythicC2Container, C2ProfileDefinition, C2Parameter};

#[tokio::main]
async fn main() {
    MythicC2Container::builder()
        .profile(C2ProfileDefinition {
            name: "my_profile".to_string(),
            description: "My C2 profile".to_string(),
            author: "@you".to_string(),
            is_p2p: false,
            is_server_routed: true,
            semver: "0.0.1".to_string(),
            agent_icon: None,
            dark_mode_agent_icon: None,
        })
        .server_binary_path("./c2_code/my_server")
        .server_folder_path("./c2_code")
        .parameters(vec![/* your C2Parameter list */])
        .on_config_check(|msg| mythic_rabbitmq::C2ConfigCheckMessageResponse {
            success: true,
            error: String::new(),
            message: "ok".to_string(),
            restart_internal_server: false,
        })
        .build()
        .start_and_run_forever()
        .await;
}
```

## Builder methods

| Method | Required | Description |
|---|---|---|
| `.profile(C2ProfileDefinition)` | Yes | Profile name, description, author, semver, p2p/server-routed flags |
| `.parameters(Vec<C2Parameter>)` | No | C2 parameters shown in the Mythic UI |
| `.server_binary_path(path)` | No | Path to the server binary the container launches on start |
| `.server_folder_path(path)` | No | Working directory for the server process (defaults to binary's parent dir) |
| `.rabbitmq_config(RabbitMQConfig)` | No | Override connection settings (defaults to env vars) |
| `.on_config_check(fn)` | No | Handler for config check RPC |
| `.on_opsec_check(fn)` | No | Handler for OPSEC check RPC |
| `.on_get_ioc(fn)` | No | Handler for get IOC RPC |
| `.on_sample_message(fn)` | No | Handler for sample message RPC |
| `.on_redirector_rules(fn)` | No | Handler for redirector rules RPC |
| `.on_host_file(fn)` | No | Handler for host file RPC |

All handlers are `fn(RequestType) -> ResponseType`. Handlers not registered use a default stub that returns `success: true` (or `false` for unsupported RPCs).

## RabbitMQ connection

Config is read from environment variables by default:

| Variable | Default |
|---|---|
| `RABBITMQ_HOST` | `mythic_rabbitmq` |
| `RABBITMQ_PORT` | `5672` |
| `RABBITMQ_USER` | `mythic_user` |
| `RABBITMQ_PASSWORD` | `mythic_password` |
| `RABBITMQ_VHOST` | `mythic_vhost` |

Override programmatically with `.rabbitmq_config(RabbitMQConfig { ... })`.

Exchange: `mythic_exchange` (direct, durable=true, auto_delete=true)
Queues: durable=false, auto_delete=true — matches the Go library's `rabbitmq/utils.go`.

## C2 parameters

Use `mythic_rabbitmq::structs::parameter_type` constants for `parameter_type`:

```rust
use mythic_rabbitmq::{C2Parameter, structs::parameter_type};

C2Parameter {
    name: "port".to_string(),
    description: "Port to listen on".to_string(),
    default_value: serde_json::json!(4444),
    parameter_type: parameter_type::NUMBER.to_string(),
    required: true,
    randomize: false,
    format_string: String::new(),
    verifier_regex: String::new(),
    is_crypto_type: false,
    choices: vec![],
    dictionary_choices: vec![],
    ui_position: 0,
}
```

Available types: `String`, `Boolean`, `Number`, `Date`, `ChooseOne`, `ChooseOneCustom`, `ChooseMultiple`, `Array`, `TypedArray`, `Dictionary`, `File`, `FileMultiple`.

For crypto parameters (e.g. AES keys), set `is_crypto_type: true` — this serializes as `"crypto_type": true` to match Mythic's wire format.

## Module layout

```
src/
  lib.rs         — MythicC2Container + builder, RPC listener wiring
  connection.rs  — RabbitMQ connect/retry, exchange + queue declaration
  sync.rs        — c2_sync RPC, mythic_rpc_c2_update_status
  rpc.rs         — Generic RPC queue listener, start/stop server, resync
  structs.rs     — All wire-format types (JSON tags match Go source exactly)
  constants.rs   — Routing key constants, exchange name, container version
  error.rs       — MythicError type
```

## Startup sequence (detail)

```
connect_with_retry()
  → send_c2_sync()            RPC to c2_sync queue, waits for success response
  → launch_server()           spawns server_binary_path as a child process
  → send_c2_update_status()   fires mythic_rpc_c2_update_status (server_running=true/false)
  → spawn RPC listeners       one tokio task per routing key, each reconnects on error
  → ctrl_c().await            blocks main task
```

## Container version

Currently targets Mythic container API `v1.4.3` (`CONTAINER_VERSION` in `constants.rs`).

## Agent (Payload Type)

`MythicAgentContainer` handles the full payload type lifecycle: on startup it sends a `pt_sync` RPC to register the payload type definition and all commands with Mythic, then spawns a build listener, per-command task listeners (create_tasking, opsec_pre/post, process_response, completion_function), an on-new-callback listener, a callbacks-alive listener, and a resync listener — then blocks on Ctrl+C.

### Minimal example

```rust
use mythic_rabbitmq::{
    MythicAgentContainer, PayloadType, Command, BuildParameter, CommandParameter,
    CommandHandlers, PayloadBuildResponse,
    structs::{
        AgentType, MessageFormat, CommandAttribute, ParameterGroupInfo,
        build_parameter_type, command_parameter_type,
    },
};

#[tokio::main]
async fn main() {
    let shell_cmd = Command {
        name: "shell".to_string(),
        description: "Execute a shell command".to_string(),
        version: 1,
        needs_admin_permissions: false,
        help_cmd: "shell <command>".to_string(),
        author: "@you".to_string(),
        script_only: false,
        supported_ui_features: vec![],
        mitre_attack_techniques: vec![],
        browser_script: None,
        attributes: CommandAttribute {
            supported_os: vec![],
            builtin: false,
            suggested_command: false,
            load_only: false,
            script_only: false,
            filter_by_build_parameter: Default::default(),
        },
        parameters: vec![CommandParameter {
            name: "command".to_string(),
            display_name: "Command".to_string(),
            cli_name: "command".to_string(),
            parameter_type: command_parameter_type::STRING.to_string(),
            description: "Shell command to run".to_string(),
            required: true,
            default_value: serde_json::Value::Null,
            verifier_regex: String::new(),
            choices: vec![],
            supported_agents: vec![],
            supported_agent_build_parameters: Default::default(),
            choices_are_all_commands: false,
            choices_are_loaded_commands: false,
            filter_command_choices_by_supported_os: false,
            dynamic_query_function: None,
            typedarray_parse_function: None,
            parameter_group_info: vec![ParameterGroupInfo {
                group_name: "Default".to_string(),
                parameter_is_required: true,
                ui_position: 0,
            }],
        }],
    };

    MythicAgentContainer::builder()
        .payload_type(PayloadType {
            name: "my_agent".to_string(),
            file_extension: "bin".to_string(),
            author: "@you".to_string(),
            supported_os: vec!["Linux".to_string(), "macOS".to_string()],
            wrapper: false,
            wrapped_payloads: vec![],
            note: "Example agent".to_string(),
            supports_dynamic_loading: false,
            build_parameters: vec![],
            build_steps: vec![],
            message_format: MessageFormat::Json,
            agent_type: AgentType::Agent,
            c2_profiles: vec![],
            mythic_encrypts: false,
            translation_container_name: String::new(),
            agent_icon: None,
            custom_rpc_functions: vec![],
        })
        .commands(vec![shell_cmd])
        .on_build(|msg| PayloadBuildResponse {
            uuid: msg.uuid.clone(),
            success: true,
            error: String::new(),
            payload: Some(b"<compiled bytes>".to_vec()),
            status: "success".to_string(),
            build_message: "built ok".to_string(),
            build_stderr: String::new(),
            build_stdout: String::new(),
            updated_command_list: None,
        })
        .command_handlers("shell", CommandHandlers {
            create_tasking: Some(|msg| {
                use mythic_rabbitmq::structs::PTTaskCreateTaskingMessageResponse;
                PTTaskCreateTaskingMessageResponse {
                    task_id: msg.task.id,
                    success: true,
                    error: String::new(),
                    command_name: msg.task.command_name,
                    params: msg.task.params,
                    display_params: msg.task.display_params,
                    stdout: String::new(),
                    stderr: String::new(),
                    completed: false,
                    status: "submitted".to_string(),
                }
            }),
            opsec_pre: None,
            opsec_post: None,
            process_response: None,
            completion_function: None,
        })
        .build()
        .start_and_run_forever()
        .await;
}
```

### Builder methods

| Method | Required | Description |
|---|---|---|
| `.payload_type(PayloadType)` | Yes | Agent name, OS list, agent type, build params, etc. |
| `.commands(Vec<Command>)` | No | Commands shown in Mythic UI |
| `.on_build(fn)` | Yes | Compiles/packages the payload; returns bytes + status |
| `.command_handlers(name, CommandHandlers)` | No | Per-command task lifecycle handlers |
| `.on_new_callback(fn)` | No | Called when a new callback checks in |
| `.on_callbacks_alive(fn)` | No | Called to check which callbacks are still active |
| `.rabbitmq_config(RabbitMQConfig)` | No | Override connection settings |

### `CommandHandlers` fields

| Field | Type | Description |
|---|---|---|
| `create_tasking` | `Option<CreateTaskingHandler>` | Validate/transform task params before execution |
| `opsec_pre` | `Option<OpsecPreHandler>` | Block a task before it runs (OPSEC gate) |
| `opsec_post` | `Option<OpsecPostHandler>` | Block a task after params are set (post-OPSEC gate) |
| `process_response` | `Option<ProcessResponseHandler>` | Custom parsing of agent response data |
| `completion_function` | `Option<CompletionFunctionHandler>` | Called when a subtask or group finishes |

### Startup sequence

```
connect_with_retry()
  → send_pt_sync()           RPC to pt_sync, waits for success response
  → spawn build listener     {pt_name}_payload_build → pt_build_response
  → spawn task listeners     per-command, per-handler-type
  → spawn resync listener
  → ctrl_c().await
```
