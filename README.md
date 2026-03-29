# AgentLink (Architecture-First MVP)

AgentLink is an **agent-native, remote-zero-install** gateway: local agent UX stays unchanged, remote host only needs `sshd`.

## Design Principles

- clear boundaries over convenience coupling
- replaceable transport backends
- platform concerns isolated from domain logic
- semantic processing as a pipeline, not ad-hoc string hacks

## Project Structure

```text
src/
  main.rs
  app/                        # application scheduling and use-case orchestration
    mod.rs
  interface/                  # external interfaces (CLI today, API later)
    cli.rs
    mod.rs
  core/                       # transport-agnostic domain logic
    agent/
      profile.rs              # per-agent behavior policy
      mod.rs
    semantic/
      cleaner.rs              # stream cleaner
      pipeline.rs             # semantic channel pipeline
      filters/                # grouped filter family
        progress.rs
        motd.rs
        mod.rs
      mod.rs
    control/
      guard.rs                # risk command gate
      emitter.rs              # status event output
      mod.rs
    ports/
      remote.rs               # RemoteTransport trait + config
      mod.rs
    mod.rs
  transport/                  # adapter implementations of ports
    ssh_cli.rs                # current backend
    mod.rs                    # transport factory
  platform/                   # cross-platform runtime and health checks
    runtime.rs
    health.rs
    mod.rs
```

## Decoupling Contract

- `app` depends on traits (`core::ports`), never on concrete backend internals.
- `core` has no dependency on transport implementation details.
- `transport/*` is isolated; current production path is `ssh_cli`.
- `platform/*` is the only place for OS/runtime-specific branching.

## Commands

```bash
cargo run -- doctor
cargo run -- bind --target user@your-vps --agent codex
cargo run -- exec --target user@your-vps --cmd "ls -la" --agent codex --clean
cargo run -- mcp-server --target user@your-vps --agent codex
cargo run -- connect --target user@your-vps --agent codex --name agentlink-vps
```

`exec` now enables SSH connection reuse by default (ControlMaster/ControlPersist).  
Disable with `--no-ssh-reuse` when needed.

## Agent UI Integration (MCP, Non-Intrusive First)

Run AgentLink as an MCP stdio server:

```bash
cargo run -- mcp-server --target user@your-vps --agent codex
```

It exposes one tool:
- `remote_exec` command execution in remote cwd
- `remote_pwd` get remote cwd
- `remote_cd` change remote cwd
- `remote_list_dir` list remote directory
- `remote_read_file` read remote UTF-8 file
- `remote_write_file` write/append remote UTF-8 file
- `remote_mkdir` create remote directory

Register it in Codex:

```bash
codex mcp add agentlink -- \
  /absolute/path/to/agentlink mcp-server --target user@your-vps --agent codex
```

Generate wiring snippets (Codex command + generic MCP JSON for Claude Code/other agents):

```bash
cargo run -- connect --target user@your-vps --agent codex --name agentlink-vps
```

Password mode is also supported for MCP:

```bash
export AGENTLINK_SSH_PASSWORD='your_password'
codex mcp add agentlink -- \
  /absolute/path/to/agentlink mcp-server --target user@your-vps --agent codex
```

Or pass directly:

```bash
codex mcp add agentlink -- \
  /absolute/path/to/agentlink mcp-server --target user@your-vps --agent codex --password 'your_password'
```

Notes:
- Password MCP mode requires `sshpass` on the local machine.
- Key-based auth still works and remains the recommended default.

## Optional Convenience Mode

`agentlink codex` is still available as a convenience wrapper, but production usage is recommended via the non-intrusive MCP path above.

## What Is Already Platform-Aware

- host platform detection (`MacOs/Linux/Windows/Unknown`)
- default terminal type policy via platform module
- SSH binary health check isolated in platform health module

## Next Evolution

1. extend MCP with remote search + structured patch application
2. add remote session state snapshots and recovery
3. add HITL risk-release gate for destructive commands
4. add binary-safe file transfer tool for large assets
