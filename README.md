# ArkLog

ArkLog 是面向 HarmonyOS 开发者的低内存实时日志终端。正式入口使用
Ratatui + Crossterm，同一份 Rust 程序支持 Windows Terminal/PowerShell 和
macOS Terminal/iTerm2，不再依赖浏览器窗口承载实时日志。

## 当前能力

- 通过 `hdc list targets -v` 发现设备，使用 `Ctrl+D` 查看和刷新完整连接列表；对在线设备按需读取 `/version/special_cust`，并以“手机代号 设备 ID”显示，读取失败时保留原 ID
- 通过 `hdc -t <device> hilog -v epoch` 启停 HiLog；Running/Stopped 意图由 Controller 持久保存，设备短暂离线或 HDC 故障不会取消自动恢复；等待设备时约每秒发现一次，采集中约每五秒核对设备列表，不以日志安静作为掉线信号
- 使用最多 50 行/首条后 100 ms 等待预算的批次和固定 8 批次通道传输，拥塞时背压而不丢行（100 ms 不是端到端硬 SLA）
- 通过单一、不区分大小写的正则表达式过滤并高亮全部非空命中，正则源和编译内存都有硬上限
- 使用 `Ctrl+F` 做不区分大小写的视图内字面量查找，支持循环定位和命中高亮
- 超宽原始记录默认保持单行，可在 HiLog 正文用左右键横向浏览；屏幕外 Find 命中会自动进入可见区
- `Ctrl+W` 可切换按终端显示格软换行；滚动、resize、Find 和 follow-latest 共用同一个内容锚点
- 正则过滤保留当前进程最近 50 条成功表达式，编辑时用上下键复用；不保存查询结果或日志快照
- 滚动离开末尾后固定当前锚点，按 `Ctrl+G` 回到最新并恢复自动跟随
- HiLog/Fault Log 共用工作区，通过 `F1`/`F2` 直接选择；普通浏览时也支持 `Tab`/`Shift+Tab` 切换，Fault Log 可刷新和浏览原始诊断
- 顶部设备、日志视图、过滤/查找和流状态共用一行紧凑控制区；没有底部状态栏，HiLog 使用完整工作区宽度
- HDC 查询带超时与输出上限，设备刷新、Fault Log 查询和停流均不阻塞界面线程
- `Clear` 不停止日志流，只开启一个新的空会话
- 可选 MCP 只读工具，让 AI Agent 查询运行中会话的 HiLog，支持时间范围、正则和分页
- 不使用 SQLite，不跨启动保留 HiLog
- 完整 HiLog 写入进程生命周期临时文件；内存只保留有界批次、索引状态和当前视窗
- release 压力探针对 100,000 行执行 50 MiB RSS 硬门禁

## 运行

可从 GitHub Releases 下载 `ArkLog-windows-x86_64.exe`、
`ArkLog-macos-x86_64` 或 `ArkLog-macos-aarch64`。
macOS 首次运行前需要赋予执行权限：

```bash
chmod +x ArkLog-macos-aarch64
./ArkLog-macos-aarch64
```

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

## AI Agent / MCP 查询

先开启运行中 TUI 的本机查询通道（默认不开启）：

```bash
ARKLOG_MCP_PORT=38765 cargo run -p arklog --release
```

Windows PowerShell：

```powershell
$env:ARKLOG_MCP_PORT = "38765"
.\ArkLog-windows-x86_64.exe
```

然后在 AI Agent 的 MCP 配置中注册同一个 ArkLog 可执行文件。以下为常见的
`mcpServers` JSON 格式；将 `command` 替换成已构建二进制的绝对路径：

```json
{
  "mcpServers": {
    "arklog": {
      "command": "/absolute/path/to/arklog",
      "args": ["--mcp"],
      "env": { "ARKLOG_MCP_PORT": "38765" }
    }
  }
}
```

`--mcp` 通过标准输入/输出提供 MCP，不启动另一个 TUI 或 HDC。它连接已经运行的
ArkLog；两端端口必须相同，适配器未设置端口时使用 `38765`。多个 TUI 可分别选择
不同端口（`1`–`65535`）；端口被占用时启动明确失败。通道只监听 `127.0.0.1`，开启后
本机进程可读取当前日志。关闭 TUI 即关闭查询通道。

提供三个只读工具：

- `arklog_session`：当前会话标识、设备 ID 和原始记录数。
- `arklog_query_logs`：查询完整原始 HiLog，不受界面正则或 Find 影响；时间可省略。
- `arklog_has_logs`：判断指定时间戳之后是否存在匹配正则的日志，返回 `{"exists": true/false}`。

`arklog_query_logs` 参数示例：

```json
{
  "start": 1793527200000,
  "end": 1793527500000,
  "regex": "error|timeout|失败",
  "limit": 100
}
```

`arklog_query_logs` 的所有参数均可省略；只传 `regex` 即可查询整个当前会话。
`start` / `end` 使用 **Unix 毫秒整数**，起止边界均包含，也可只传其中一个。
采集命令使用 `hilog -v epoch`，原始日志中的 `1793527200.001` 表示 Unix 秒，
查询时准确转换为 `1793527200001` 毫秒；无需推断年份或时区，可跨年查询。
带时间条件时跳过无法解析时间的记录，省略时间则保留这些记录。
正则默认不区分大小写，使用 Rust regex 语法，不支持回溯引用或环视；错误表达式、
非整数时间和失效游标作为 MCP 工具错误返回。

`arklog_has_logs` 参数示例：

```json
{
  "after": 1793527200000,
  "regex": "error|timeout|失败"
}
```

其中 `regex` 必传，`after` 可省略；省略时检查整个当前会话。传入 `after` 时，只判断
日志时间戳 **严格大于** 该毫秒值的记录。结果是 `{"exists": true}` 或
`{"exists": false}`，不返回日志正文。工具检查请求时的固定快照，不等待后续新日志；
会继续扫描后续分页，确认查完才返回 `false`。扫描超过约 5 秒仍未得出结论或发生
读取错误时返回工具错误，不把未完成的查询当成 `false`。

`arklog_query_logs` 的结果含 `session_id`、`device_id`、`raw_count`、`until`、`records` 和 `next_cursor`；
每条记录含从 `0` 开始的原始 `index` 和完整 `text`。按采集顺序返回，默认每页 100 条，
最多 1000 条。继续查询时保留原筛选条件，将返回的 `next_cursor` 作为 `cursor`，并携带
第一页的 `session_id` 和 `until`，即可固定查询边界，不混入后续追加的日志。
`next_cursor` 为 `null` 表示结束；空页也可能带继续游标。

扫描在后台线程执行，每页最多扫描 100,000 条或约 8 MiB，扫描循环每条记录之间检查
250 ms 时间预算；返回正文最多 256 KiB。不会截断原始记录；单条命中记录超过返回预算
或单条原始记录超过 1 MiB 时明确返回工具错误。Clear、设备切换、重新启动采集
（包括重连）或退出后不提供旧会话的新查询，旧会话游标须重新获取。已取得文件快照的请求仍可完成。

完整行为与验证见 [MCP 查询规格](docs/specs/agent-log-query.md)。

## 键盘操作

- `F1` / `F2`：直接打开 HiLog / Fault Log；在过滤、查找或设备列表中也可使用，未应用的输入草稿不会自动提交
- `Tab` / `Shift+Tab`：普通浏览时切换 HiLog/Fault Log；Windows Terminal 的 `Ctrl+Tab` / `Ctrl+Shift+Tab` 切换的是终端自身标签页
- `Ctrl+D`：打开/关闭设备连接列表
- `Ctrl+S`：启动/停止 HiLog；刷新或启停过渡期间再次按下可取消/反转待执行操作
- `Ctrl+R`：刷新设备连接列表或当前 Fault Log
- `←`/`→`：在 HiLog 正文横向浏览；在设备列表中切换设备
- `Home`：在 HiLog 正文回到第 0 列
- `Ctrl+W`：仅在 HiLog 正文切换 Soft Wrap
- `Ctrl+E`：编辑正则过滤；编辑时 `↑`/`↓` 浏览更旧/更新的成功表达式
- `Ctrl+F`：打开视图内查找；`Enter`/`Shift+Enter` 前后定位
- `Ctrl+N`/`Ctrl+P`：下一个/上一个查找命中
- `↑`/`↓`：滚动 HiLog 或选择 Fault Log 条目
- `PageUp`/`PageDown`：滚动 HiLog 或当前 Fault Log 原始诊断
- `Ctrl+G`：回到最新
- `Ctrl+L`：清空当前 HiLog 会话
- `Ctrl+Q`：安全停止日志流并退出

正则和查找输入框支持桌面文本编辑习惯：Windows 使用 `Ctrl`，macOS 使用 `⌘`；
`A/C/X/V` 为全选、复制、剪切、粘贴，`Z/Y` 与 `Shift+Z` 为撤销/重做。
方向键、`Home`、`End` 可移动光标，配合 `Shift` 选择；Windows 用 `Ctrl`
按词移动/删除，macOS 用 `Option` 按词、`⌘` 按整行。输入、粘贴及 32 级
撤销历史均有界，不会随日志增长。

日志正文的鼠标选择和复制仍由终端处理：macOS 使用 `⌘C`，Windows Terminal
通常使用 `Ctrl+Shift+C`。`Ctrl+C` 不再作为 ArkLog 的退出键。
Windows 经典控制台运行期间会关闭 Quick Edit，避免鼠标误选文本后暂停控制台 I/O、
导致全部快捷键失去响应；退出时恢复启动前的控制台输入设置。需要鼠标选区复制时，
建议使用 Windows Terminal。

普通单字符不会触发命令；在正则和查找输入框中可直接输入文本。

正则历史只存在于当前 ArkLog 进程，容量为最近 50 条。仅按 `Enter` 成功验证并应用的
非空表达式会记录；合法零命中表达式也会记录。表达式按原文精确去重，不会 trim、
lowercase 或预编译保存。选择历史项只替换编辑内容，仍需再次按 `Enter` 才会应用；
`Esc` 取消当前草稿。Clear、设备切换和重连不清除输入历史，退出后不持久化。

默认显示策略仍是一条原始记录占一行。横向偏移按终端 cell 计算，左右溢出符只表示
对应方向确有隐藏内容；它们和软换行续行符都不是原文，不参与存储、匹配或复制。
Soft Wrap 按 Unicode 字素边界折行，无空格 URL/JSON 也可继续阅读；Tab 仅在显示时按
4-cell tab stop 展开，磁盘中的 UTF-8 原始记录保持不变。终端宿主的屏幕选区复制仍只
复制可见选区；需要完整记录时应从原始会话数据取得，不能把屏幕选区误认为完整记录。

## 执行诊断日志

ArkLog 默认把应用执行轨迹写入系统临时目录：Windows 为
`%TEMP%\arklog-execution.log`，macOS 为 `$TMPDIR/arklog-execution.log`。退出后终端会
打印实际路径。需要把日志放到容易找到的位置时，可以在启动前指定：

```powershell
$env:ARKLOG_EXECUTION_LOG = "$PWD\arklog-execution.log"
.\ArkLog-windows-x86_64.exe
```

```bash
ARKLOG_EXECUTION_LOG="$PWD/arklog-execution.log" ./ArkLog-macos-aarch64
```

该文件记录 `Ctrl+S` 等命令、连接/流/Fault 状态变化、时间戳和错误，最大 1 MiB；
不会记录 HiLog、Fault Log 正文、正则/查找内容或剪贴板内容。系统或 HDC 错误可能带有
设备标识或本机路径，公开上传前请检查。报告真实设备问题时，请同时附上复现步骤、
ArkLog 版本和这份执行日志。

## 显示建议

ArkLog 使用 Catppuccin Mocha 语义色和标准 Unicode 圆角边框，不依赖 Nerd Font。
终端程序不能修改宿主字体，因此建议选择无连字的等宽字体，避免正则与原始日志字符
被视觉合并：

双击 Windows 发布版 `.exe` 时，ArkLog 保持在 Windows 分配的当前控制台中，避免强制
转交给 `wt.exe` 后任务栏只显示 Windows Terminal 图标。如果明确希望使用 Windows
Terminal 的字体渲染，可执行 `ArkLog-windows-x86_64.exe --windows-terminal`。如果系统已将
Windows Terminal 设为默认终端宿主，窗口和任务栏图标仍由终端宿主控制。

- Windows Terminal：`Cascadia Mono`，12–13 pt（Windows Terminal 默认自带）
- macOS Terminal：`Menlo`，12–13 pt
- iTerm2：`JetBrains Mono`，12–13 pt，并关闭 ligatures

普通 HiLog 只有 Level 字段使用语义色：错误/致命为红色，警告为黄色，信息为绿色，
调试为蓝色，verbose 为弱化灰。时间、行号、PID/TID、Tag 和 Message 始终使用中性正文色；
只有已激活正则过滤或 Find 的实际命中字符可以局部覆盖样式。正则命中使用黄色底色，
查找命中使用紫色下划线，当前查找行只增加低对比度表面背景，不改变其他字段颜色。
颜色只改变显示样式，不改写、裁剪或重新排序原始日志。

横向溢出、软换行续行标记、Tab stop、横移步长及其样式同样由语义设计 Token 提供。

全部颜色、样式、边框、选中/命中状态及属于视觉设计的布局尺寸和间距由
`crates/arklog-tui/src/theme.rs` 以语义 token 统一提供；渲染模块不持有 raw RGB 或
未命名视觉 magic value。`Constraint::Fill(1)` 等结构语义保持为布局代码本身。

## 验证

```bash
pnpm test
pnpm build
pnpm memory:check
cargo check -p arklog
```

`memory:check` 会构建 release 二进制，写入并读取 100,000 条模拟日志，采样进程
峰值 RSS，同时保留 2 MiB Fault Log；达到或超过 50 MiB 时返回失败。GitHub CI
会在 macOS 和 Windows 上执行相同门禁。

## 结构

```text
crates/arklog-core/          HDC 发现、命令、无损批处理和进程生命周期
crates/arklog-tui/           Ratatui 应用、临时会话仓库、状态、界面和内存门禁
docs/specs/                  行为与架构规格
src/、src-tauri/、tests/     迁移期间保留的旧 React/Tauri 实现与回归证据
```

正式 Cargo workspace 只包含 `arklog-core` 和 `arklog-tui`。旧 Web/Tauri 代码不参与
默认构建，可通过 `legacy:*` 脚本单独验证，待跨平台验收后删除。

## 已知限制

- 当前支持的 HDC/HiLog 命令面没有经过验证的服务端 `since`、cursor 或 attach
  boundary。普通 `hilog` 可能先输出设备缓冲区中的既有记录；ArkLog 不用电脑本地时间
  猜测边界，也不会默认执行会清除全设备日志缓存的 `hilog -r`。严格 LiveOnly 请求通过
  `DeviceLogStartError::UnsupportedLiveStart` 明确失败，不会静默回退后声称“只看新日志”。
- `SessionLogStore` 对固定正则和 Find 的新日志 append 是增量的；但用户调用
  `set_filter` 或 `set_find` 更换查询时，历史索引仍在调用线程同步重建。该卡顿风险留作
  独立后续问题，本轮没有引入 QueryWorker 或第二套存储服务。
