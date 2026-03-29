use crate::core::agent::AgentProfile;
use crate::core::ports::{RemoteTransport, TransportConfig};
use crate::core::semantic::{CleanerConfig, pump_with_semantic_channel};
use crate::platform::runtime;
use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::io::{self, AsyncWriteExt};
use tokio::process::Command;

pub struct SshCliTransport {
    config: TransportConfig,
    profile: AgentProfile,
}

impl SshCliTransport {
    pub fn new(config: TransportConfig, profile: AgentProfile) -> Self {
        Self { config, profile }
    }

    fn base_command(&self) -> Command {
        let mut cmd = Command::new(&self.config.ssh_bin);
        cmd.env("TERM", runtime::default_term_type());
        cmd.args(self.profile.transport_ssh_args());
        cmd.args(&self.config.extra_ssh_args);
        cmd.arg(&self.config.target);
        cmd
    }
}

#[async_trait]
impl RemoteTransport for SshCliTransport {
    async fn bind_interactive(&self) -> Result<Option<i32>> {
        let mut cmd = self.base_command();
        cmd.arg("-tt");
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd
            .spawn()
            .with_context(|| format!("failed to spawn ssh to {}", self.config.target))?;

        let mut child_stdin = child.stdin.take().context("missing child stdin")?;
        let child_stdout = child.stdout.take().context("missing child stdout")?;
        let child_stderr = child.stderr.take().context("missing child stderr")?;

        let stdin_to_child = tokio::spawn(async move {
            let mut input = io::stdin();
            io::copy(&mut input, &mut child_stdin).await
        });

        let out_task = tokio::spawn(async move {
            pump_with_semantic_channel(child_stdout, io::stdout(), CleanerConfig::disabled()).await
        });

        let err_task = tokio::spawn(async move {
            pump_with_semantic_channel(child_stderr, io::stderr(), CleanerConfig::disabled()).await
        });

        let status = child.wait().await?;

        let _ = stdin_to_child.await;
        out_task.await??;
        err_task.await??;

        Ok(status.code())
    }

    async fn exec_command(&self, remote_cmd: &str, clean: bool) -> Result<Option<i32>> {
        let mut cmd = self.base_command();
        cmd.arg(remote_cmd);
        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd
            .spawn()
            .with_context(|| format!("failed to execute command on {}", self.config.target))?;

        let stdout = child.stdout.take().context("missing stdout")?;
        let stderr = child.stderr.take().context("missing stderr")?;

        let cleaner_cfg = self.profile.cleaner_config(clean);

        let out_task = tokio::spawn(async move {
            pump_with_semantic_channel(stdout, io::stdout(), cleaner_cfg).await
        });

        let err_task = tokio::spawn(async move {
            pump_with_semantic_channel(stderr, io::stderr(), cleaner_cfg).await
        });

        let status = child.wait().await?;
        out_task.await??;
        err_task.await??;

        if !status.success() {
            let _ = io::stderr()
                .write_all(
                    format!(
                        "[agentlink] command failed on {} (profile: {})\n",
                        self.config.target,
                        self.profile.name()
                    )
                    .as_bytes(),
                )
                .await;
        }

        Ok(status.code())
    }
}
