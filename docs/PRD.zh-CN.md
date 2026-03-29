# A-Tunnel PRD（架构优先版）

## 1. 产品目标

A-Tunnel 让本地 Agent 直接操作远端服务器，同时保持：

- 本地使用体验不变
- 远端零安装（仅 `sshd`）
- 架构可演进（传输替换不影响上层）

## 2. 架构原则

- 先边界后功能：模块职责明确，避免横向耦合
- 端口-适配器：核心逻辑依赖 trait，不依赖实现
- 平台隔离：OS 差异仅在 `platform` 层处理
- 同类聚合：语义过滤器按目录分组管理

## 3. 分层设计

1. `app`（调度层）
- 编排 use-case，不做传输细节

2. `interface`（接口层）
- CLI 输入解析

3. `core`（领域层）
- `agent`：不同 Agent 的策略配置
- `semantic`：流式清洗与语义 pipeline
- `control`：风险拦截与状态事件
- `ports`：传输抽象 trait

4. `transport`（适配层）
- 当前 `ssh_cli`
- 后续 `russh` / `portable-pty` / `sftp`

5. `platform`（平台层）
- 平台识别、默认终端参数、健康检查

## 4. 关于“不同 Agent 是否要不同 Channel”

结论：

- 传输 Channel 通常统一（都走 SSH/PTY 数据平面）
- 语义策略按 Agent 区分（Cleaner/过滤规则/输出容忍度）

即：**Channel 通用，Policy 差异化**。

## 5. 当前可运行能力

- `bind`：持续会话
- `exec`：一次命令
- `--agent`：`codex / claudecode / aider / generic`
- 高危命令默认拦截
- 状态事件可输出到 UDS

## 6. 下一阶段（不破坏上层）

- 在 `transport` 新增 `russh` 实现并接入工厂
- 在 `transport` 增加 PTY 与 SFTP 适配器目录
- `app/interface/core` 尽量保持零改动
