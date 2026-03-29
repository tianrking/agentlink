use crate::core::agent::AgentProfile;
use crate::core::ports::TransportConfig;
use anyhow::{Context, Result};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::{Value, json};
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
struct SessionState {
    cwd: String,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            cwd: "~".to_string(),
        }
    }
}

pub fn run_stdio_server(config: TransportConfig, profile: AgentProfile) -> Result<()> {
    if config.ssh_password.is_some() {
        ensure_sshpass_available()?;
    }

    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    let mut state = SessionState::default();

    while let Some(raw) = read_message(&mut reader)? {
        let msg: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let is_notification = msg.get("id").is_none();
        let method = msg
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();

        if is_notification && (method == "notifications/initialized" || method == "initialized") {
            continue;
        }

        let Some(id) = msg.get("id").cloned() else {
            continue;
        };

        let response = match method {
            "initialize" => initialize_response(id),
            "tools/list" => tools_list_response(id),
            "tools/call" => handle_tools_call(id, &msg, &config, &profile, &mut state),
            "resources/list" => resources_list_response(id, &state),
            "resources/templates/list" => resources_templates_list_response(id),
            "resources/read" => handle_resources_read(id, &msg, &config, &profile, &mut state),
            _ => method_not_found(id, method),
        };

        write_message(&mut writer, &response)?;
    }

    Ok(())
}

fn initialize_response(id: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {},
                "resources": {}
            },
            "serverInfo": {
                "name": "agentlink-mcp",
                "version": env!("CARGO_PKG_VERSION")
            }
        }
    })
}

fn resources_list_response(id: Value, state: &SessionState) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "resources": [
                {
                    "uri": "agentlink://pwd",
                    "name": "Remote Working Directory",
                    "description": "Current remote cwd for this MCP session",
                    "mimeType": "text/plain",
                    "text": state.cwd
                }
            ]
        }
    })
}

fn resources_templates_list_response(id: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "resourceTemplates": [
                {
                    "uriTemplate": "agentlink://exec/{cmd_b64url}",
                    "name": "Remote Exec",
                    "description": "Execute remote shell command in current cwd; cmd_b64url is URL-safe base64 of UTF-8 command",
                    "mimeType": "text/plain"
                },
                {
                    "uriTemplate": "agentlink://cd/{path_b64url}",
                    "name": "Remote Cd",
                    "description": "Change remote cwd; path_b64url is URL-safe base64 of UTF-8 path",
                    "mimeType": "text/plain"
                },
                {
                    "uriTemplate": "agentlink://list_dir/{path_b64url}",
                    "name": "Remote List Dir",
                    "description": "List remote directory; path_b64url is URL-safe base64 of UTF-8 path",
                    "mimeType": "text/plain"
                },
                {
                    "uriTemplate": "agentlink://read_file/{path_b64url}",
                    "name": "Remote Read File",
                    "description": "Read remote UTF-8 file; path_b64url is URL-safe base64 of UTF-8 path",
                    "mimeType": "text/plain"
                }
            ]
        }
    })
}

fn handle_resources_read(
    id: Value,
    msg: &Value,
    config: &TransportConfig,
    profile: &AgentProfile,
    state: &mut SessionState,
) -> Value {
    let Some(uri) = msg.pointer("/params/uri").and_then(Value::as_str) else {
        return invalid_args(id, "missing required parameter: params.uri");
    };

    let result = if uri == "agentlink://pwd" {
        Ok((0, state.cwd.clone(), String::new()))
    } else if let Some(v) = uri.strip_prefix("agentlink://exec/") {
        match decode_b64url(v) {
            Ok(cmd) => remote_exec(config, profile, state, &cmd),
            Err(err) => Err(err),
        }
    } else if let Some(v) = uri.strip_prefix("agentlink://cd/") {
        match decode_b64url(v) {
            Ok(path) => remote_cd(config, profile, state, &path),
            Err(err) => Err(err),
        }
    } else if let Some(v) = uri.strip_prefix("agentlink://list_dir/") {
        match decode_b64url(v) {
            Ok(path) => remote_list_dir(config, profile, state, &path),
            Err(err) => Err(err),
        }
    } else if let Some(v) = uri.strip_prefix("agentlink://read_file/") {
        match decode_b64url(v) {
            Ok(path) => remote_read_file(config, profile, state, &path),
            Err(err) => Err(err),
        }
    } else {
        Err(anyhow::anyhow!("unsupported resource uri: {uri}"))
    };

    match result {
        Ok((code, out, err)) => {
            let mut text = String::new();
            if !out.trim().is_empty() {
                text.push_str("stdout:\n");
                text.push_str(&out);
                if !out.ends_with('\n') {
                    text.push('\n');
                }
            }
            if !err.trim().is_empty() {
                text.push_str("stderr:\n");
                text.push_str(&err);
                if !err.ends_with('\n') {
                    text.push('\n');
                }
            }
            text.push_str(&format!("exit_code: {code}\n"));

            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "contents": [
                        {
                            "uri": uri,
                            "mimeType": "text/plain",
                            "text": text
                        }
                    ]
                }
            })
        }
        Err(err) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32000,
                "message": format!("resource read failed: {err}")
            }
        }),
    }
}

fn tools_list_response(id: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "tools": [
                {
                    "name": "remote_exec",
                    "description": "Execute shell command on remote server in current remote cwd",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "cmd": { "type": "string", "description": "Remote shell command" }
                        },
                        "required": ["cmd"]
                    }
                },
                {
                    "name": "remote_pwd",
                    "description": "Get current remote working directory",
                    "inputSchema": { "type": "object", "properties": {} }
                },
                {
                    "name": "remote_cd",
                    "description": "Change current remote working directory",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string" }
                        },
                        "required": ["path"]
                    }
                },
                {
                    "name": "remote_list_dir",
                    "description": "List directory entries on remote server",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Default: ." }
                        }
                    }
                },
                {
                    "name": "remote_read_file",
                    "description": "Read remote file as UTF-8 text",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string" }
                        },
                        "required": ["path"]
                    }
                },
                {
                    "name": "remote_write_file",
                    "description": "Write UTF-8 text into remote file",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string" },
                            "content": { "type": "string" },
                            "append": { "type": "boolean", "description": "Default: false" }
                        },
                        "required": ["path", "content"]
                    }
                },
                {
                    "name": "remote_mkdir",
                    "description": "Create remote directory recursively",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": { "type": "string" }
                        },
                        "required": ["path"]
                    }
                }
            ]
        }
    })
}

fn handle_tools_call(
    id: Value,
    msg: &Value,
    config: &TransportConfig,
    profile: &AgentProfile,
    state: &mut SessionState,
) -> Value {
    let name = msg
        .pointer("/params/name")
        .and_then(Value::as_str)
        .unwrap_or_default();

    let args = msg.pointer("/params/arguments").unwrap_or(&Value::Null);

    let result = match name {
        "remote_exec" => {
            let Some(cmd) = args.get("cmd").and_then(Value::as_str) else {
                return invalid_args(id, "missing required argument: cmd");
            };
            remote_exec(config, profile, state, cmd)
        }
        "remote_pwd" => Ok((0, state.cwd.clone(), String::new())),
        "remote_cd" => {
            let Some(path) = args.get("path").and_then(Value::as_str) else {
                return invalid_args(id, "missing required argument: path");
            };
            remote_cd(config, profile, state, path)
        }
        "remote_list_dir" => {
            let path = args.get("path").and_then(Value::as_str).unwrap_or(".");
            remote_list_dir(config, profile, state, path)
        }
        "remote_read_file" => {
            let Some(path) = args.get("path").and_then(Value::as_str) else {
                return invalid_args(id, "missing required argument: path");
            };
            remote_read_file(config, profile, state, path)
        }
        "remote_write_file" => {
            let Some(path) = args.get("path").and_then(Value::as_str) else {
                return invalid_args(id, "missing required argument: path");
            };
            let Some(content) = args.get("content").and_then(Value::as_str) else {
                return invalid_args(id, "missing required argument: content");
            };
            let append = args.get("append").and_then(Value::as_bool).unwrap_or(false);
            remote_write_file(config, profile, state, path, content, append)
        }
        "remote_mkdir" => {
            let Some(path) = args.get("path").and_then(Value::as_str) else {
                return invalid_args(id, "missing required argument: path");
            };
            remote_mkdir(config, profile, state, path)
        }
        _ => return invalid_args(id, &format!("unknown tool: {name}")),
    };

    match result {
        Ok((code, out, err)) => tool_result(id, code, &out, &err),
        Err(err) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [
                    { "type": "text", "text": format!("tool execution failed: {err}") }
                ],
                "isError": true
            }
        }),
    }
}

fn tool_result(id: Value, code: i32, out: &str, err: &str) -> Value {
    let mut text = String::new();
    if !out.trim().is_empty() {
        text.push_str("stdout:\n");
        text.push_str(out);
        if !out.ends_with('\n') {
            text.push('\n');
        }
    }
    if !err.trim().is_empty() {
        text.push_str("stderr:\n");
        text.push_str(err);
        if !err.ends_with('\n') {
            text.push('\n');
        }
    }
    text.push_str(&format!("exit_code: {code}\n"));

    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [{ "type": "text", "text": text }],
            "isError": code != 0
        }
    })
}

fn invalid_args(id: Value, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": -32602, "message": message }
    })
}

fn method_not_found(id: Value, method: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": -32601, "message": format!("method not found: {method}") }
    })
}

fn remote_exec(
    config: &TransportConfig,
    profile: &AgentProfile,
    state: &SessionState,
    cmd: &str,
) -> Result<(i32, String, String)> {
    run_remote_command(config, profile, state, cmd)
}

fn remote_cd(
    config: &TransportConfig,
    profile: &AgentProfile,
    state: &mut SessionState,
    path: &str,
) -> Result<(i32, String, String)> {
    let script = format!("cd {} && pwd -P", shell_quote(path));
    let (code, out, err) = run_remote_command(config, profile, state, &script)?;
    if code == 0 {
        let new_cwd = out.lines().next().unwrap_or("").trim();
        if !new_cwd.is_empty() {
            state.cwd = new_cwd.to_string();
        }
    }
    Ok((code, out, err))
}

fn remote_list_dir(
    config: &TransportConfig,
    profile: &AgentProfile,
    state: &SessionState,
    path: &str,
) -> Result<(i32, String, String)> {
    let script = format!("ls -la -- {}", shell_quote(path));
    run_remote_command(config, profile, state, &script)
}

fn remote_read_file(
    config: &TransportConfig,
    profile: &AgentProfile,
    state: &SessionState,
    path: &str,
) -> Result<(i32, String, String)> {
    let sentinel = "__AGENTLINK_NOT_FILE__";
    let script = format!(
        "if [ -f {p} ]; then (base64 -w0 -- {p} 2>/dev/null || base64 -- {p} | tr -d '\\n'); else echo {s}; exit 3; fi",
        p = shell_quote(path),
        s = shell_quote(sentinel),
    );

    let (code, out, err) = run_remote_command(config, profile, state, &script)?;
    if code != 0 {
        return Ok((code, out, err));
    }

    let payload = out.trim();
    if payload == sentinel {
        return Ok((3, String::new(), "path is not a regular file".to_string()));
    }

    let decoded = STANDARD
        .decode(payload)
        .context("failed to decode remote base64 file content")?;
    let text = String::from_utf8_lossy(&decoded).to_string();
    Ok((0, text, String::new()))
}

fn remote_write_file(
    config: &TransportConfig,
    profile: &AgentProfile,
    state: &SessionState,
    path: &str,
    content: &str,
    append: bool,
) -> Result<(i32, String, String)> {
    let encoded = STANDARD.encode(content.as_bytes());
    let op = if append { ">>" } else { ">" };
    let script = format!(
        "mkdir -p -- \"$(dirname -- {p})\" && printf %s {b} | base64 -d {op} {p}",
        p = shell_quote(path),
        b = shell_quote(&encoded),
        op = op,
    );

    run_remote_command(config, profile, state, &script)
}

fn remote_mkdir(
    config: &TransportConfig,
    profile: &AgentProfile,
    state: &SessionState,
    path: &str,
) -> Result<(i32, String, String)> {
    let script = format!("mkdir -p -- {}", shell_quote(path));
    run_remote_command(config, profile, state, &script)
}

fn run_remote_command(
    config: &TransportConfig,
    profile: &AgentProfile,
    state: &SessionState,
    script: &str,
) -> Result<(i32, String, String)> {
    let remote_cmd = format!("cd {} && {}", shell_quote(&state.cwd), script);
    exec_over_ssh(config, profile, &remote_cmd)
}

fn exec_over_ssh(
    config: &TransportConfig,
    profile: &AgentProfile,
    remote_cmd: &str,
) -> Result<(i32, String, String)> {
    let mut cmd = if let Some(password) = &config.ssh_password {
        let mut c = Command::new("sshpass");
        c.arg("-e");
        c.arg(&config.ssh_bin);
        c.env("SSHPASS", password);
        c
    } else {
        Command::new(&config.ssh_bin)
    };

    cmd.args(profile.transport_ssh_args());
    if !has_ssh_opt(&config.extra_ssh_args, "StrictHostKeyChecking") {
        cmd.arg("-o").arg("StrictHostKeyChecking=accept-new");
    }
    if !has_ssh_opt(&config.extra_ssh_args, "ConnectTimeout") {
        cmd.arg("-o").arg("ConnectTimeout=10");
    }
    if config.ssh_password.is_none() && !has_ssh_opt(&config.extra_ssh_args, "BatchMode") {
        cmd.arg("-o").arg("BatchMode=yes");
    }
    if config.ssh_password.is_some() {
        if !has_ssh_opt(&config.extra_ssh_args, "PreferredAuthentications") {
            cmd.arg("-o").arg("PreferredAuthentications=password");
        }
        if !has_ssh_opt(&config.extra_ssh_args, "PubkeyAuthentication") {
            cmd.arg("-o").arg("PubkeyAuthentication=no");
        }
        if !has_ssh_opt(&config.extra_ssh_args, "KbdInteractiveAuthentication") {
            cmd.arg("-o").arg("KbdInteractiveAuthentication=no");
        }
    }

    if config.ssh_reuse
        && !has_ssh_opt(&config.extra_ssh_args, "ControlMaster")
        && !has_ssh_opt(&config.extra_ssh_args, "ControlPath")
        && !has_ssh_opt(&config.extra_ssh_args, "ControlPersist")
        && !config.extra_ssh_args.iter().any(|arg| arg == "-S")
    {
        cmd.arg("-o").arg("ControlMaster=auto");
        cmd.arg("-o").arg(format!(
            "ControlPersist={}s",
            config.ssh_control_persist_secs
        ));
        cmd.arg("-o").arg("ControlPath=/tmp/agentlink-%C");
    }

    cmd.args(&config.extra_ssh_args);
    cmd.arg(&config.target);
    cmd.arg(remote_cmd);

    let output = cmd
        .output()
        .with_context(|| format!("failed to execute ssh for target {}", config.target))?;

    let code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok((code, stdout, stderr))
}

fn ensure_sshpass_available() -> Result<()> {
    let status = Command::new("sshpass")
        .arg("-V")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("failed to start sshpass; install sshpass for password MCP mode")?;
    if !status.success() {
        anyhow::bail!("sshpass is required for password-based MCP mode");
    }
    Ok(())
}

fn shell_quote(text: &str) -> String {
    if text.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", text.replace('\'', "'\"'\"'"))
}

fn has_ssh_opt(extra_ssh_args: &[String], key: &str) -> bool {
    extra_ssh_args.iter().any(|arg| {
        arg == key || arg.starts_with(&format!("{key}=")) || arg.contains(&format!("{key}="))
    })
}

fn decode_b64url(input: &str) -> Result<String> {
    let bytes = URL_SAFE_NO_PAD
        .decode(input)
        .context("invalid base64url segment in resource uri")?;
    Ok(String::from_utf8(bytes).context("decoded value is not valid UTF-8")?)
}

fn read_message(reader: &mut dyn BufRead) -> Result<Option<String>> {
    let mut content_length: Option<usize> = None;

    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            return Ok(None);
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }

        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(rest.trim().parse::<usize>()?);
        }
    }

    let len = content_length.context("missing Content-Length header")?;
    let mut buf = vec![0_u8; len];
    reader.read_exact(&mut buf)?;
    let body = String::from_utf8(buf).context("invalid utf-8 body")?;
    Ok(Some(body))
}

fn write_message(writer: &mut dyn Write, body: &Value) -> Result<()> {
    let text = serde_json::to_string(body)?;
    write!(writer, "Content-Length: {}\r\n\r\n{}", text.len(), text)?;
    writer.flush()?;
    Ok(())
}
