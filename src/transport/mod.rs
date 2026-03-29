pub mod ssh_cli;

use crate::core::agent::AgentProfile;
use crate::core::ports::{RemoteTransport, TransportConfig};

#[derive(Debug, Clone, Copy, Default)]
pub enum TransportKind {
    #[default]
    SshCli,
}

pub fn build_transport(
    kind: TransportKind,
    config: TransportConfig,
    profile: AgentProfile,
) -> Box<dyn RemoteTransport> {
    match kind {
        TransportKind::SshCli => Box::new(ssh_cli::SshCliTransport::new(config, profile)),
    }
}
