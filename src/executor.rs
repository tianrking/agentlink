use anyhow::{Context, Result, anyhow, bail};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct ExecResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone)]
pub struct TmuxConfig {
    pub pane: String,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone)]
pub struct SshConfig {
    pub target: String,
    pub ssh_bin: String,
    pub password: Option<String>,
    pub extra_ssh_args: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum ExecTarget {
    Tmux(TmuxConfig),
    Ssh(SshConfig),
}

pub fn run_exec(target: ExecTarget, cmd: &str) -> Result<ExecResult> {
    match target {
        ExecTarget::Tmux(cfg) => run_tmux_exec(&cfg, cmd),
        ExecTarget::Ssh(cfg) => run_ssh_exec(&cfg, cmd),
    }
}

fn run_tmux_exec(cfg: &TmuxConfig, cmd: &str) -> Result<ExecResult> {
    ensure_tmux_available()?;

    let token = unique_token();
    let start_marker = format!("__AGENTLINK_START_{}__", token);
    let end_marker = format!("__AGENTLINK_END_{}__", token);

    // Sent as literal to tmux pane.
    let wrapped = format!(
        "printf '%s\\n' '{start}'; {{ {cmd}; }}; __al_ec=$?; printf '%s:%s\\n' '{end}' \"$__al_ec\"",
        start = start_marker,
        end = end_marker,
        cmd = cmd
    );

    send_tmux_literal(&cfg.pane, &wrapped)?;
    send_tmux_enter(&cfg.pane)?;

    let timeout = Duration::from_millis(cfg.timeout_ms);
    let started = Instant::now();

    loop {
        if started.elapsed() > timeout {
            bail!("tmux command timed out after {} ms", cfg.timeout_ms);
        }

        let pane_dump = capture_tmux_pane(&cfg.pane)?;
        if let Some(result) = parse_marked_output(&pane_dump, &start_marker, &end_marker)? {
            return Ok(result);
        }

        thread::sleep(Duration::from_millis(120));
    }
}

fn run_ssh_exec(cfg: &SshConfig, cmd: &str) -> Result<ExecResult> {
    let mut c = if let Some(password) = &cfg.password {
        let mut s = Command::new("sshpass");
        s.arg("-e");
        s.arg(&cfg.ssh_bin);
        s.env("SSHPASS", password);
        s
    } else {
        Command::new(&cfg.ssh_bin)
    };

    c.arg("-o").arg("StrictHostKeyChecking=accept-new");
    c.arg("-o").arg("ConnectTimeout=10");
    c.args(&cfg.extra_ssh_args);
    c.arg(&cfg.target);
    c.arg(cmd);

    let out = c
        .output()
        .with_context(|| format!("failed to execute ssh command on {}", cfg.target))?;

    Ok(ExecResult {
        exit_code: out.status.code().unwrap_or(1),
        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
    })
}

fn ensure_tmux_available() -> Result<()> {
    let status = Command::new("tmux")
        .arg("-V")
        .status()
        .context("failed to run tmux -V")?;
    if !status.success() {
        bail!("tmux is required for tmux backend");
    }
    Ok(())
}

fn send_tmux_literal(pane: &str, content: &str) -> Result<()> {
    let status = Command::new("tmux")
        .arg("send-keys")
        .arg("-t")
        .arg(pane)
        .arg("-l")
        .arg(content)
        .status()
        .context("failed to send tmux literal keys")?;
    if !status.success() {
        bail!("tmux send-keys failed for pane {pane}");
    }
    Ok(())
}

fn send_tmux_enter(pane: &str) -> Result<()> {
    let status = Command::new("tmux")
        .arg("send-keys")
        .arg("-t")
        .arg(pane)
        .arg("C-m")
        .status()
        .context("failed to send Enter to tmux")?;
    if !status.success() {
        bail!("tmux send-keys C-m failed for pane {pane}");
    }
    Ok(())
}

fn capture_tmux_pane(pane: &str) -> Result<String> {
    let out = Command::new("tmux")
        .arg("capture-pane")
        .arg("-p")
        .arg("-J")
        .arg("-S")
        .arg("-200000")
        .arg("-t")
        .arg(pane)
        .output()
        .context("failed to capture tmux pane")?;
    if !out.status.success() {
        bail!("tmux capture-pane failed for pane {pane}");
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn parse_marked_output(
    pane_dump: &str,
    start_marker: &str,
    end_marker: &str,
) -> Result<Option<ExecResult>> {
    let start_idx = match pane_dump.rfind(start_marker) {
        Some(i) => i,
        None => return Ok(None),
    };

    let after_start = &pane_dump[start_idx + start_marker.len()..];
    let end_pos_rel = match after_start.rfind(end_marker) {
        Some(i) => i,
        None => return Ok(None),
    };

    let between = &after_start[..end_pos_rel];
    let after_end = &after_start[end_pos_rel + end_marker.len()..];
    let mut after_end_lines = after_end.lines();
    let code_line = after_end_lines.next().unwrap_or_default();

    let exit_code = if let Some(rest) = code_line.strip_prefix(':') {
        rest.trim().parse::<i32>().unwrap_or(1)
    } else {
        return Err(anyhow!("failed to parse tmux command exit code"));
    };

    let stdout = between.trim_matches('\n').to_string();
    Ok(Some(ExecResult {
        exit_code,
        stdout: if stdout.is_empty() {
            String::new()
        } else {
            format!("{stdout}\n")
        },
        stderr: String::new(),
    }))
}

fn unique_token() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    nanos.to_string()
}
