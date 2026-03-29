use crate::core::agent::AgentProfile;
use crate::core::control::emitter::{Event, StatusEmitter};
use crate::core::control::guard;
use crate::core::ports::TransportConfig;
use crate::interface::cli::{Cli, Commands};
use crate::platform::{health, runtime};
use crate::transport::{self, TransportKind};
use anyhow::Result;

pub async fn run(cli: Cli) -> Result<i32> {
    match cli.command {
        Commands::Doctor { ssh_bin } => {
            health::check_ssh_binary(&ssh_bin).await?;
            println!(
                "a-tunnel doctor: ssh binary is available ({ssh_bin}), platform={:?}",
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
            };
            let transport = transport::build_transport(TransportKind::SshCli, config, profile);
            handle_result(transport.bind_interactive().await, &mut emitter).await
        }
        Commands::Exec {
            target,
            cmd,
            agent,
            ssh_bin,
            clean,
            allow_high_risk,
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
            };
            let transport = transport::build_transport(TransportKind::SshCli, config, profile);
            handle_result(transport.exec_command(&cmd, clean).await, &mut emitter).await
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
