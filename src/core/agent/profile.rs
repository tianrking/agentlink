use crate::core::semantic::CleanerConfig;
use clap::ValueEnum;

#[derive(Debug, Clone, ValueEnum)]
pub enum AgentKind {
    Codex,
    Claudecode,
    Aider,
    Generic,
}

#[derive(Debug, Clone)]
pub struct AgentProfile {
    kind: AgentKind,
}

impl AgentProfile {
    pub fn new(kind: AgentKind) -> Self {
        Self { kind }
    }

    pub fn name(&self) -> &'static str {
        match self.kind {
            AgentKind::Codex => "codex",
            AgentKind::Claudecode => "claudecode",
            AgentKind::Aider => "aider",
            AgentKind::Generic => "generic",
        }
    }

    pub fn cleaner_config(&self, clean: bool) -> CleanerConfig {
        if !clean {
            return CleanerConfig::disabled();
        }

        match self.kind {
            AgentKind::Claudecode => CleanerConfig {
                enabled: true,
                strip_ansi: false,
                drop_progress: false,
                drop_motd: false,
            },
            AgentKind::Codex | AgentKind::Aider => CleanerConfig {
                enabled: true,
                strip_ansi: true,
                drop_progress: true,
                drop_motd: true,
            },
            AgentKind::Generic => CleanerConfig {
                enabled: true,
                strip_ansi: true,
                drop_progress: true,
                drop_motd: false,
            },
        }
    }

    pub fn transport_ssh_args(&self) -> Vec<String> {
        match self.kind {
            AgentKind::Codex | AgentKind::Aider => vec![
                "-o".to_string(),
                "LogLevel=ERROR".to_string(),
                "-o".to_string(),
                "ServerAliveInterval=20".to_string(),
            ],
            AgentKind::Claudecode => vec![
                "-o".to_string(),
                "ServerAliveInterval=20".to_string(),
                "-o".to_string(),
                "ServerAliveCountMax=3".to_string(),
            ],
            AgentKind::Generic => vec!["-o".to_string(), "ServerAliveInterval=20".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentKind, AgentProfile};

    #[test]
    fn codex_profile_enables_strict_cleaning() {
        let profile = AgentProfile::new(AgentKind::Codex);
        let cfg = profile.cleaner_config(true);
        assert!(cfg.enabled);
        assert!(cfg.strip_ansi);
        assert!(cfg.drop_progress);
        assert!(cfg.drop_motd);
    }

    #[test]
    fn claudecode_profile_keeps_ansi_and_progress() {
        let profile = AgentProfile::new(AgentKind::Claudecode);
        let cfg = profile.cleaner_config(true);
        assert!(cfg.enabled);
        assert!(!cfg.strip_ansi);
        assert!(!cfg.drop_progress);
        assert!(!cfg.drop_motd);
    }

    #[test]
    fn clean_disabled_returns_disabled_config() {
        let profile = AgentProfile::new(AgentKind::Generic);
        let cfg = profile.cleaner_config(false);
        assert!(!cfg.enabled);
    }
}
