use anyhow::{Result, bail};
use regex::Regex;

pub fn reject_if_high_risk(cmd: &str) -> Result<()> {
    let patterns = [
        r"(?i)\brm\s+-rf\b",
        r"(?i)\bdd\s+if=",
        r"(?i)\bmkfs(\.|\s)",
        r"(?i)\bshutdown\b",
        r"(?i)\breboot\b",
        r"(?i)\bdrop\s+database\b",
        r"(?i)\btruncate\s+table\b",
    ];

    for pat in patterns {
        let re = Regex::new(pat)?;
        if re.is_match(cmd) {
            bail!("blocked high-risk command; rerun with --allow-high-risk if intended: {cmd}");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::reject_if_high_risk;

    #[test]
    fn allows_safe_command() {
        assert!(reject_if_high_risk("ls -la").is_ok());
    }

    #[test]
    fn blocks_rm_rf() {
        assert!(reject_if_high_risk("rm -rf /tmp/foo").is_err());
    }

    #[test]
    fn blocks_drop_database() {
        assert!(reject_if_high_risk("DROP DATABASE prod").is_err());
    }
}
