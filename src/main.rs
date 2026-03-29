mod cli;
mod executor;
mod mcp;

use anyhow::Result;
use clap::Parser;
use cli::{BackendKind, Cli, Commands};
use executor::{ExecResult, ExecTarget, SshConfig, TmuxConfig, run_exec};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::TmuxExec {
            pane,
            cmd,
            timeout_ms,
        } => {
            let result = run_exec(ExecTarget::Tmux(TmuxConfig { pane, timeout_ms }), &cmd)?;
            print_result(&result);
        }
        Commands::SshExec {
            target,
            cmd,
            ssh_bin,
            password,
            password_env,
            extra_ssh_args,
        } => {
            let resolved_password = password.or_else(|| std::env::var(&password_env).ok());
            let result = run_exec(
                ExecTarget::Ssh(SshConfig {
                    target,
                    ssh_bin,
                    password: resolved_password,
                    extra_ssh_args,
                }),
                &cmd,
            )?;
            print_result(&result);
        }
        Commands::McpServer {
            backend,
            pane,
            target,
            ssh_bin,
            password,
            password_env,
            extra_ssh_args,
            timeout_ms,
        } => {
            let exec_target = match backend {
                BackendKind::Tmux => {
                    let pane = pane
                        .ok_or_else(|| anyhow::anyhow!("--pane is required for --backend tmux"))?;
                    ExecTarget::Tmux(TmuxConfig { pane, timeout_ms })
                }
                BackendKind::Ssh => {
                    let target = target
                        .ok_or_else(|| anyhow::anyhow!("--target is required for --backend ssh"))?;
                    let resolved_password = password.or_else(|| std::env::var(&password_env).ok());
                    ExecTarget::Ssh(SshConfig {
                        target,
                        ssh_bin,
                        password: resolved_password,
                        extra_ssh_args,
                    })
                }
            };

            mcp::run_stdio_server(exec_target)?;
        }
    }

    Ok(())
}

fn print_result(result: &ExecResult) {
    if !result.stdout.is_empty() {
        print!("{}", result.stdout);
        if !result.stdout.ends_with('\n') {
            println!();
        }
    }
    if !result.stderr.is_empty() {
        eprint!("{}", result.stderr);
        if !result.stderr.ends_with('\n') {
            eprintln!();
        }
    }
    println!("exit_code: {}", result.exit_code);
}
