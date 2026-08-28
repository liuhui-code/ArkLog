# ArkLog

ArkLog 是从 ArkLine 的 Device Log 能力中拆出的独立 HarmonyOS 日志工作台。首个可运行切片提供 HDC 设备发现、HiLog 启停和实时批量日志显示。

## 当前能力

- 通过 `hdc list targets -v` 发现设备并标记连接状态
- 通过 `hdc -t <device> hilog` 启动实时日志流
- 将输出按最多 50 行或 100 ms 的窗口批量传给界面
- 停止日志流时终止并回收 HDC 子进程
- 界面仅保留最近 2,000 行，避免无限增长
- 使用 `ARKLOG_HDC_PATH` 指定非 PATH 中的 HDC 可执行文件

SQLite 持久化、历史查询、Fault Log 和 ArkLine 启动入口属于后续切片，尚未包含。

## 开发

要求 Node.js、pnpm、Rust 1.85+、Tauri 2 的平台依赖，以及可用的 HarmonyOS `hdc`。

```bash
pnpm install
pnpm test
cargo test --workspace
pnpm tauri dev
```

如果 HDC 不在 PATH：

```bash
ARKLOG_HDC_PATH=/path/to/hdc pnpm tauri dev
```

生产前端构建与桌面类型检查：

```bash
pnpm build
cargo check -p arklog
```

## 结构

```text
src/                         React 界面与稳定的 ArkLogApi
src-tauri/                   Tauri 命令、状态和事件薄适配
crates/arklog-core/          不依赖 Tauri 的 HDC 与日志流核心
tests/                       前端公共接口测试
crates/arklog-core/tests/    Rust 公共接口与进程生命周期测试
```

React 不直接散落调用 Tauri；`src/tauri-api.ts` 是唯一的前端宿主适配边界。`arklog-core` 不依赖窗口或事件框架，因此后续可以复用在 CLI、sidecar 或其他宿主中。

## 来源与边界

项目的第一阶段行为从 ArkLine Device Log 领域拆分而来，实施时的 ArkLine 参考提交为 `e8e91334ecc71db172be01d623c79cafb731987d`。本项目是独立代码库，不修改或复制 ArkLine 当前工作区中的未提交改动。
