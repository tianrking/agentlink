use crate::core::agent::AgentKind;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "agentlink", about = "Agent-native SSH tunnel gateway", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Validate SSH binary availability.
    Doctor {
        #[arg(long, default_value = "ssh")]
        ssh_bin: String,
    },
    /// Interactive mode: make remote shell look local for Agent.
    Bind {
        #[arg(long)]
        target: String,
        #[arg(long, value_enum, default_value = "generic")]
        agent: AgentKind,
        #[arg(long, default_value = "ssh")]
        ssh_bin: String,
        /// Extra raw args passed to ssh, e.g. --extra-ssh-args "-p" --extra-ssh-args "2222"
        #[arg(long = "extra-ssh-args")]
        extra_ssh_args: Vec<String>,
        /// Optional Unix socket path for status events.
        #[arg(long)]
        status_socket: Option<PathBuf>,
    },
    /// One-shot command execution through SSH.
    Exec {
        #[arg(long)]
        target: String,
        #[arg(long)]
        cmd: String,
        #[arg(long, value_enum, default_value = "generic")]
        agent: AgentKind,
        #[arg(long, default_value = "ssh")]
        ssh_bin: String,
        /// Enable semantic cleaning pipeline.
        #[arg(long, default_value_t = false)]
        clean: bool,
        /// Allow high-risk commands (rm -rf, DROP DATABASE, etc.)
        #[arg(long, default_value_t = false)]
        allow_high_risk: bool,
        /// Disable SSH connection reuse for one-shot exec.
        #[arg(long, default_value_t = false)]
        no_ssh_reuse: bool,
        /// SSH control socket keep-alive time in seconds for exec reuse.
        #[arg(long, default_value_t = 600)]
        ssh_control_persist_secs: u32,
        #[arg(long = "extra-ssh-args")]
        extra_ssh_args: Vec<String>,
        #[arg(long)]
        status_socket: Option<PathBuf>,
    },
    /// Run MCP stdio server so Agent UIs can execute commands remotely via tools.
    McpServer {
        #[arg(long)]
        target: String,
        #[arg(long, value_enum, default_value = "codex")]
        agent: AgentKind,
        #[arg(long, default_value = "ssh")]
        ssh_bin: String,
        #[arg(long, default_value_t = false)]
        no_ssh_reuse: bool,
        #[arg(long, default_value_t = 600)]
        ssh_control_persist_secs: u32,
        /// Direct SSH password for MCP mode (insecure in shell history).
        #[arg(long, conflicts_with = "password_env")]
        password: Option<String>,
        /// Read SSH password from this environment variable (default: AGENTLINK_SSH_PASSWORD).
        #[arg(
            long,
            default_value = "AGENTLINK_SSH_PASSWORD",
            conflicts_with = "password"
        )]
        password_env: String,
        #[arg(long = "extra-ssh-args")]
        extra_ssh_args: Vec<String>,
    },
}
