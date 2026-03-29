use anyhow::{Context, Result, bail};
use tokio::process::Command;

pub async fn check_ssh_binary(ssh_bin: &str) -> Result<()> {
    let status = Command::new(ssh_bin)
        .arg("-V")
        .status()
        .await
        .with_context(|| format!("failed to execute {ssh_bin} -V"))?;

    if !status.success() {
        bail!("ssh binary check failed");
    }

    Ok(())
}
