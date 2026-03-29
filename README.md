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
- `transport/*` can be swapped (`ssh_cli` -> `russh`) without changing `core`.
- `platform/*` is the only place for OS/runtime-specific branching.

## Commands

```bash
cargo run -- doctor
cargo run -- bind --target user@your-vps --agent codex
cargo run -- exec --target user@your-vps --cmd "ls -la" --agent codex --clean
```

`exec` now enables SSH connection reuse by default (ControlMaster/ControlPersist).  
Disable with `--no-ssh-reuse` when needed.
`ssh-cli` is already the default transport, so `--transport` is optional.

## What Is Already Platform-Aware

- host platform detection (`MacOs/Linux/Windows/Unknown`)
- default terminal type policy via platform module
- SSH binary health check isolated in platform health module

## Next Evolution

1. add `russh` backend implementing `RemoteTransport`
2. add `portable-pty` adapter under `transport/pty_*`
3. add SFTP cache adapter under `transport/sftp_*`
4. keep `app/interface/core` unchanged during backend upgrades
5. transport switch available now: `--transport ssh-cli|russh` (`russh` scaffold is wired, implementation pending)
