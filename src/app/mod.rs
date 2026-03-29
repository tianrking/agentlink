mod mcp;

use crate::core::agent::AgentProfile;
use crate::core::control::emitter::{Event, StatusEmitter};
use crate::core::control::guard;
use crate::core::ports::{RemoteTransport, TransportConfig};
use crate::interface::cli::{Cli, Commands};
use crate::platform::{health, runtime};
use crate::transport::ssh_cli::SshCliTransport;
use anyhow::Result;

pub async fn run(cli: Cli) -> Result<i32> {
    match cli.command {
        Commands::Doctor { ssh_bin } => {
            health::check_ssh_binary(&ssh_bin).await?;
            println!(
                "agentlink doctor: ssh binary is available ({ssh_bin}), platform={:?}",
                runtime::detect_platform()
            );
            Ok(0)
        }
        Commands::Bind {
            target,
            agent,
            ssh_bin,
            extra_ssh_args,
            status_socket,
        } => {
            let profile = AgentProfile::new(agent);
            let mut emitter = StatusEmitter::new(status_socket);
            let config = TransportConfig {
                target,
                ssh_bin,
                extra_ssh_args,
                ssh_reuse: false,
                ssh_control_persist_secs: 0,
                ssh_password: None,
            };
            let transport = SshCliTransport::new(config, profile);
            handle_result(transport.bind_interactive().await, &mut emitter).await
        }
        Commands::Exec {
            target,
            cmd,
            agent,
            ssh_bin,
            clean,
            allow_high_risk,
            no_ssh_reuse,
            ssh_control_persist_secs,
            extra_ssh_args,
            status_socket,
        } => {
            if !allow_high_risk {
                guard::reject_if_high_risk(&cmd)?;
            }

            let profile = AgentProfile::new(agent);
            let mut emitter = StatusEmitter::new(status_socket);
            let config = TransportConfig {
                target,
                ssh_bin,
                extra_ssh_args,
                ssh_reuse: !no_ssh_reuse,
                ssh_control_persist_secs,
                ssh_password: None,
            };
            let transport = SshCliTransport::new(config, profile);
            handle_result(transport.exec_command(&cmd, clean).await, &mut emitter).await
        }
        Commands::McpServer {
            target,
            agent,
            ssh_bin,
            no_ssh_reuse,
            ssh_control_persist_secs,
            password,
            password_env,
            extra_ssh_args,
        } => {
            let resolved_password = match password {
                Some(p) => Some(p),
                None => std::env::var(&password_env).ok(),
            };

            let profile = AgentProfile::new(agent);
            let config = TransportConfig {
                target,
                ssh_bin,
                extra_ssh_args,
                ssh_reuse: !no_ssh_reuse,
                ssh_control_persist_secs,
                ssh_password: resolved_password,
            };
            mcp::run_stdio_server(config, profile)?;
            Ok(0)
        }
    }
}

async fn handle_result(result: Result<Option<i32>>, emitter: &mut StatusEmitter) -> Result<i32> {
    match result {
        Ok(code) => {
            emitter.emit(Event::ExitCode { code }).await;
            Ok(code.unwrap_or(1))
        }
        Err(err) => {
            emitter
                .emit(Event::Error {
                    message: err.to_string(),
                })
                .await;
            Err(err)
        }
    }
}
