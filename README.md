# AgentLink

Minimal, non-intrusive bridge that lets local agents operate a remote environment.

## What it does

- Keep Codex / Claude Code running locally.
- Route command execution to:
- an existing local `tmux` pane (that pane can already be SSH-connected to your VPS), or
- direct SSH target.
- Expose one MCP tool: `remote_exec`.

## Commands

### 1) Execute through an existing tmux pane

```bash
agentlink tmux-exec --pane dev:0.0 --cmd "hostname && uname -a"
```

### 2) Execute directly over SSH

```bash
agentlink ssh-exec --target root@hk2.w0x7ce.eu --cmd "hostname && uname -a"
```

Password mode:

```bash
export AGENTLINK_SSH_PASSWORD='255=ff'
agentlink ssh-exec --target root@hk2.w0x7ce.eu --cmd "hostname" --password-env AGENTLINK_SSH_PASSWORD
```

### 3) Run MCP server (for local agents)

tmux backend:

```bash
agentlink mcp-server --backend tmux --pane dev:0.0
```

ssh backend:

```bash
agentlink mcp-server --backend ssh --target root@hk2.w0x7ce.eu --password-env AGENTLINK_SSH_PASSWORD
```

Then register with your agent as a stdio MCP server.

## Design

- `cli.rs`: command surface
- `executor.rs`: `tmux` / `ssh` backends
- `mcp.rs`: minimal MCP stdio protocol (`remote_exec`)

No auto-modification of Codex/Claude config.
