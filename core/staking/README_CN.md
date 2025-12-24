（此文件说明 `ya_staking` crate 的构建、测试、运行与部署步骤，适用于仓库路径 `backend/core/staking`）

**概览**

- **目的**：`ya_staking`（crate 名称）实现了 D-role staking 的核心逻辑（SQLite 存储）：`StakingState`、`ProviderRecord`、`EventRecord` 及相关操作（register/stake/reward/slash/withdraw/get_events）。
- **位置**：仓库内路径为 [backend/core/staking](backend/core/staking)

**环境与依赖**

- 需要 Rust toolchain（建议 stable/latest），Cargo。仓库是一个 workspace，通常在仓库根（`backend`）构建。
- 依赖：`rusqlite`（bundled）、`r2d2`/`r2d2_sqlite`、`anyhow`、`chrono`、`serde`。

**构建**

- 在仓库根直接构建该 crate（workspace 可识别）：

```bash
# 在仓库根或 backend 目录运行
cargo build -p ya_staking
```

**测试**

- 运行该 crate 的单元/集成测试：

```bash
# 仅运行 ya_staking 的测试
cargo test -p ya_staking

# 或运行仓库中专门的集成测试（示例）
cargo test --manifest-path backend/Cargo.toml --test staking_unit
```

测试说明：仓库包含 `backend/tests/staking_unit.rs`，它已改为直接使用库 API（不依赖 HTTP 路由），便于在 CI 中快速执行。

**在代码中使用 `ya_staking`**

- 在其它 crate（例如 Verifier）中添加依赖（workspace 内推荐使用相对路径）：

```toml
[dependencies]
ya_staking = { path = "core/staking" }
```

- 最小使用示例：

```rust
use std::path::Path;
use ya_staking::StakingState;

fn example() -> anyhow::Result<()> {
    let state = StakingState::new(Path::new("./data"))?;
    let rec = state.register("node_test", 5.0)?;
    let rec = state.stake("node_test", 3.0)?;
    let rec = state.reward("node_test", 2.0)?;
    Ok(())
}
```

**数据文件与运维**

- DB 文件：`<base_dir>/staking.db`（由 `StakingState::new` 创建）。
- WAL 模式已启用，推荐把 `base_dir` 指向持久化目录并做好权限/备份。
- 查看 DB：

```bash
sqlite3 ./data/staking.db "SELECT provider_id, stake, rewards, slashed, updated_at FROM providers;"
```

**部署建议**

- 作为库：部署依赖该库的服务（如 Verifier/yagna），确保 `base_dir` 可写并持久化。
- 作为独立服务：打包适配层为可执行，使用 systemd 或容器部署，挂载数据卷并暴露端口。

**调试与常见问题**

- Windows 文件锁：在替换可执行或 DB 文件前确保进程已停止。
- 错误处理：库返回 `anyhow::Error`，上层服务应记录并映射为合适的 HTTP 状态码。

**常用命令**

- 构建： `cargo build -p ya_staking`
- 测试： `cargo test -p ya_staking` 或 `cargo test --manifest-path backend/Cargo.toml --test staking_unit`

**参考**

- 代码： backend/core/staking
- 测试示例： backend/tests/staking_unit.rs


backend\data\staking.db 是在项目目录下手动创建/初始化的本地 SQLite 数据库 —— 它不是正在运行的 yagna 服务默认使用的数据库（服务默认写到用户配置目录，如日志显示的 staking.db）。所以目前你在项目下看到的文件是“测试/演示用”的本地 DB，，但不是服务运行时的默认位置，除非你把服务配置为使用它。

yagna 启动时会选择一个数据目录（日志中显示 Data directory），默认是用户的 Roaming AppData，所以真实运行时的 staking 数据通常在该目录下。在仓库里手工运行 sqlite3 并向 staking.db 写入数据 —— 这是独立的数据库副本，用于本地测试或开发