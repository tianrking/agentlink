pub mod russh;
pub mod ssh_cli;

use crate::core::agent::AgentProfile;
use crate::core::ports::{RemoteTransport, TransportConfig};
use clap::ValueEnum;

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum TransportKind {
    #[default]
    SshCli,
    Russh,
}

pub fn build_transport(
    kind: TransportKind,
    config: TransportConfig,
    profile: AgentProfile,
) -> Box<dyn RemoteTransport> {
    match kind {
        TransportKind::SshCli => Box::new(ssh_cli::SshCliTransport::new(config, profile)),
        TransportKind::Russh => Box::new(russh::RusshTransport::new(config, profile)),
    }
}
