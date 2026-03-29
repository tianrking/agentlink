use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "agentlink",
    version,
    about = "Simple local-agent bridge for remote terminals"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Execute through an existing tmux pane (that pane may already SSH to a VPS).
    TmuxExec {
        #[arg(long, help = "tmux pane target, e.g. dev:0.0")]
        pane: String,
        #[arg(long)]
        cmd: String,
        #[arg(long, default_value_t = 120_000)]
        timeout_ms: u64,
    },
    /// Execute directly over SSH.
    SshExec {
        #[arg(long)]
        target: String,
        #[arg(long)]
        cmd: String,
        #[arg(long, default_value = "ssh")]
        ssh_bin: String,
        #[arg(long, conflicts_with = "password_env")]
        password: Option<String>,
        #[arg(
            long,
            default_value = "AGENTLINK_SSH_PASSWORD",
            conflicts_with = "password"
        )]
        password_env: String,
        #[arg(long = "extra-ssh-args")]
        extra_ssh_args: Vec<String>,
    },
    /// MCP stdio server for Codex/Claude/other local agents.
    McpServer {
        #[arg(long, value_enum)]
        backend: BackendKind,
        #[arg(long, help = "required when backend=tmux")]
        pane: Option<String>,
        #[arg(long, help = "required when backend=ssh")]
        target: Option<String>,
        #[arg(long, default_value = "ssh")]
        ssh_bin: String,
        #[arg(long, conflicts_with = "password_env")]
        password: Option<String>,
        #[arg(
            long,
            default_value = "AGENTLINK_SSH_PASSWORD",
            conflicts_with = "password"
        )]
        password_env: String,
        #[arg(long = "extra-ssh-args")]
        extra_ssh_args: Vec<String>,
        #[arg(long, default_value_t = 120_000)]
        timeout_ms: u64,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum BackendKind {
    Tmux,
    Ssh,
}
