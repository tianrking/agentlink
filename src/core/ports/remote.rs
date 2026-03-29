use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub target: String,
    pub ssh_bin: String,
    pub extra_ssh_args: Vec<String>,
    pub ssh_reuse: bool,
    pub ssh_control_persist_secs: u32,
    pub ssh_password: Option<String>,
}

#[async_trait]
pub trait RemoteTransport: Send {
    async fn bind_interactive(&self) -> Result<Option<i32>>;
    async fn exec_command(&self, remote_cmd: &str, clean: bool) -> Result<Option<i32>>;
}
