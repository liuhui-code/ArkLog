# ArkLog

ArkLog 是面向 HarmonyOS 开发者的低内存实时日志终端。正式入口使用
Ratatui + Crossterm，同一份 Rust 程序支持 Windows Terminal/PowerShell 和
macOS Terminal/iTerm2，不再依赖浏览器窗口承载实时日志。

## 当前能力

- 通过 `hdc list targets -v` 发现和切换设备
- 通过 `hdc -t <device> hilog` 启停实时 HiLog
- 使用最多 50 行/100 ms 的批次和固定 8 批次通道传输，拥塞时背压而不丢行
- 通过单一正则表达式过滤并高亮全部非空命中，正则源和编译内存都有硬上限
- 使用 `Ctrl+F`/`Command+F` 做视图内字面量查找，支持循环定位和命中高亮
- 滚动离开末尾后固定当前锚点，按 `G` 回到最新并恢复自动跟随
- HiLog/Fault Log 共用工作区，通过 `Tab` 切换；Fault Log 可刷新和浏览原始诊断
- `Clear` 不停止日志流，只开启一个新的空会话
- 不使用 SQLite，不跨启动保留 HiLog
- 完整 HiLog 写入进程生命周期临时文件；内存只保留有界批次、索引状态和当前视窗
- release 压力探针对 100,000 行执行 50 MiB RSS 硬门禁

## 运行

要求 Rust 1.85+ 和可用的 HarmonyOS `hdc`：

```bash
pnpm dev
```

也可以完全绕过 Node.js：

```bash
cargo run -p arklog --release
```

如果 HDC 不在 PATH：

```bash
ARKLOG_HDC_PATH=/path/to/hdc cargo run -p arklog --release
```

Windows PowerShell：

```powershell
$env:ARKLOG_HDC_PATH = "C:\path\to\hdc.exe"
cargo run -p arklog --release
```

## 键盘操作

- `Tab`：切换 HiLog/Fault Log
- `S`：启动/停止 HiLog
- `R`：刷新设备或当前 Fault Log
- `←`/`→`：切换设备
- `/`：编辑正则过滤
- `Ctrl+F`：打开视图内查找；`Enter`/`Shift+Enter` 前后定位
- `n`/`N`：下一个/上一个查找命中
- `↑`/`↓`、`PageUp`/`PageDown`：滚动日志或选择 Fault Log
- `G`：回到最新
- `C`：清空当前 HiLog 会话
- `Q`：退出

## 验证

```bash
pnpm test
pnpm build
pnpm memory:check
cargo check -p arklog
```

`memory:check` 会构建 release 二进制，写入并读取 100,000 条模拟日志，采样进程
峰值 RSS；达到或超过 50 MiB 时返回失败。

## 结构

```text
crates/arklog-core/          HDC 发现、命令、无损批处理和进程生命周期
crates/arklog-tui/           Ratatui 应用、临时会话仓库、状态、界面和内存门禁
docs/specs/                  行为与架构规格
src/、src-tauri/、tests/     迁移期间保留的旧 React/Tauri 实现与回归证据
```

正式 Cargo workspace 只包含 `arklog-core` 和 `arklog-tui`。旧 Web/Tauri 代码不参与
默认构建，可通过 `legacy:*` 脚本单独验证，待跨平台验收后删除。
