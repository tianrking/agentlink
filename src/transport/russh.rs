use crate::core::agent::AgentProfile;
use crate::core::ports::{RemoteTransport, TransportConfig};
use anyhow::{Result, bail};
use async_trait::async_trait;

pub struct RusshTransport {
    config: TransportConfig,
    profile: AgentProfile,
}

impl RusshTransport {
    pub fn new(config: TransportConfig, profile: AgentProfile) -> Self {
        Self { config, profile }
    }
}

#[async_trait]
impl RemoteTransport for RusshTransport {
    async fn bind_interactive(&self) -> Result<Option<i32>> {
        bail!(
            "russh transport is scaffolded but not implemented yet (target={}, profile={})",
            self.config.target,
            self.profile.name()
        )
    }

    async fn exec_command(&self, _remote_cmd: &str, _clean: bool) -> Result<Option<i32>> {
        bail!(
            "russh transport is scaffolded but not implemented yet (target={}, profile={})",
            self.config.target,
            self.profile.name()
        )
    }
}
