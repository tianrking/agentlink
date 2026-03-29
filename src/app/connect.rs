use crate::core::agent::AgentKind;
use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct ConnectGuide {
    pub target: String,
    pub agent: AgentKind,
    pub name: String,
    pub ssh_bin: String,
    pub no_ssh_reuse: bool,
    pub ssh_control_persist_secs: u32,
    pub password_env: String,
    pub extra_ssh_args: Vec<String>,
}

pub fn print_connect_guide(cfg: ConnectGuide) -> Result<()> {
    let self_exe = std::env::current_exe().context("failed to resolve current executable path")?;
    let exe = self_exe.display().to_string();

    let mut args = vec![
        "mcp-server".to_string(),
        "--target".to_string(),
        cfg.target.clone(),
        "--agent".to_string(),
        cfg.agent.as_str().to_string(),
        "--ssh-bin".to_string(),
        cfg.ssh_bin.clone(),
        "--ssh-control-persist-secs".to_string(),
        cfg.ssh_control_persist_secs.to_string(),
        "--password-env".to_string(),
        cfg.password_env.clone(),
    ];

    if cfg.no_ssh_reuse {
        args.push("--no-ssh-reuse".to_string());
    }
    for extra in &cfg.extra_ssh_args {
        args.push("--extra-ssh-args".to_string());
        args.push(extra.clone());
    }

    let joined_args = shell_join(&args);

    println!("AgentLink Non-Intrusive Connect Guide");
    println!();
    println!("1) MCP server command (stdio):");
    println!("{exe} {joined_args}");
    println!();
    println!("2) Codex registration (manual, no auto-modification):");
    println!("codex mcp add {} -- {exe} {joined_args}", cfg.name);
    println!();
    println!("3) Generic MCP stdio JSON snippet (for Claude Code / other agents):");
    println!("{{");
    println!("  \"name\": \"{}\",", cfg.name);
    println!("  \"command\": \"{}\",", escape_json_string(&exe));
    println!("  \"args\": [{}],", json_string_array(&args));
    println!(
        "  \"env\": {{ \"{}\": \"<your_ssh_password_or_leave_empty_for_key_auth>\" }}",
        cfg.password_env
    );
    println!("}}");
    println!();
    println!("4) Startup check:");
    println!("export {}='your_password'   # optional", cfg.password_env);
    println!(
        "{exe} mcp-server --target {} --agent {} --ssh-bin {}",
        shell_escape(&cfg.target),
        cfg.agent.as_str(),
        shell_escape(&cfg.ssh_bin)
    );

    Ok(())
}

fn shell_join(args: &[String]) -> String {
    args.iter().map(|a| shell_escape(a)).collect::<Vec<_>>().join(" ")
}

fn shell_escape(input: &str) -> String {
    if input.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", input.replace('\'', "'\"'\"'"))
}

fn escape_json_string(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('\"', "\\\"")
        .replace('\n', "\\n")
}

fn json_string_array(items: &[String]) -> String {
    items
        .iter()
        .map(|s| format!("\"{}\"", escape_json_string(s)))
        .collect::<Vec<_>>()
        .join(", ")
}
