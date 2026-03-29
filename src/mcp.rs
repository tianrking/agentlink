use crate::executor::{ExecTarget, run_exec};
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::io::{self, BufRead, BufReader,  Write};

pub fn run_stdio_server(target: ExecTarget) -> Result<()> {
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let mut writer = stdout.lock();

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
            "tools/call" => handle_tools_call(id, &msg, &target),
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
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "agentlink-mcp",
                "version": env!("CARGO_PKG_VERSION")
            }
        }
    })
}

fn tools_list_response(id: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "tools": [
                {
                    "name": "remote_exec",
                    "description": "Execute shell command on configured backend and return stdout/stderr/exit code",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "cmd": { "type": "string" }
                        },
                        "required": ["cmd"]
                    }
                }
            ]
        }
    })
}

fn handle_tools_call(id: Value, msg: &Value, target: &ExecTarget) -> Value {
    let name = msg
        .pointer("/params/name")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if name != "remote_exec" {
        return invalid_args(id, &format!("unknown tool: {name}"));
    }

    let Some(cmd) = msg.pointer("/params/arguments/cmd").and_then(Value::as_str) else {
        return invalid_args(id, "missing required argument: cmd");
    };

    match run_exec(target.clone(), cmd) {
        Ok(res) => {
            let mut text = String::new();
            if !res.stdout.trim().is_empty() {
                text.push_str("stdout:\n");
                text.push_str(&res.stdout);
                if !res.stdout.ends_with('\n') {
                    text.push('\n');
                }
            }
            if !res.stderr.trim().is_empty() {
                text.push_str("stderr:\n");
                text.push_str(&res.stderr);
                if !res.stderr.ends_with('\n') {
                    text.push('\n');
                }
            }
            text.push_str(&format!("exit_code: {}\n", res.exit_code));

            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [{ "type": "text", "text": text }],
                    "isError": res.exit_code != 0
                }
            })
        }
        Err(err) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [{ "type": "text", "text": format!("tool execution failed: {err}") }],
                "isError": true
            }
        }),
    }
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
