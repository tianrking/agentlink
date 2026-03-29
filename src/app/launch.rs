use anyhow::{Context, Result, bail};
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct CodexLaunchConfig {
    pub target: String,
    pub ssh_bin: String,
    pub no_ssh_reuse: bool,
    pub ssh_control_persist_secs: u32,
    pub password: Option<String>,
    pub password_env: String,
    pub extra_ssh_args: Vec<String>,
    pub prompt: Option<String>,
}

pub fn launch_codex(cfg: CodexLaunchConfig) -> Result<i32> {
    let self_exe = std::env::current_exe().context("failed to resolve current executable path")?;
    let server_name = format!("agentlink-{}", sanitize_name(&cfg.target));

    // Best-effort cleanup in case a stale config already exists.
    let _ = Command::new("codex")
        .arg("mcp")
        .arg("remove")
        .arg(&server_name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    let mut add = Command::new("codex");
    add.arg("mcp")
        .arg("add")
        .arg(&server_name)
        .arg("--")
        .arg(&self_exe)
        .arg("mcp-server")
        .arg("--target")
        .arg(&cfg.target)
        .arg("--agent")
        .arg("codex")
        .arg("--ssh-bin")
        .arg(&cfg.ssh_bin)
        .arg("--ssh-control-persist-secs")
        .arg(cfg.ssh_control_persist_secs.to_string())
        .arg("--password-env")
        .arg(&cfg.password_env);

    if cfg.no_ssh_reuse {
        add.arg("--no-ssh-reuse");
    }

    for a in &cfg.extra_ssh_args {
        add.arg("--extra-ssh-args").arg(a);
    }

    let status = add.status().context("failed to run `codex mcp add`")?;
    if !status.success() {
        bail!("failed to register AgentLink MCP server into Codex")
    }

    let mut codex = Command::new("codex");
    if let Some(p) = &cfg.prompt {
        codex.arg(p);
    } else {
        codex.arg(default_remote_first_prompt());
    }
    if let Some(password) = &cfg.password {
        codex.env(&cfg.password_env, password);
    }
    codex.stdin(Stdio::inherit());
    codex.stdout(Stdio::inherit());
    codex.stderr(Stdio::inherit());

    let exit = codex
        .status()
        .context("failed to launch interactive Codex")?
        .code()
        .unwrap_or(1);

    // Best-effort cleanup when Codex exits.
    let _ = Command::new("codex")
        .arg("mcp")
        .arg("remove")
        .arg(&server_name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    Ok(exit)
}

fn default_remote_first_prompt() -> &'static str {
    "You are connected through AgentLink. For all command execution and file operations, use AgentLink MCP tools (remote_exec, remote_cd, remote_list_dir, remote_read_file, remote_write_file, remote_mkdir). Do not use local shell commands unless explicitly requested."
}

fn sanitize_name(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('-');
        }
    }
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').to_string()
}
