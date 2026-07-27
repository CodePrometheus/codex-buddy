<p align="center">
  <img src="docs/logo.png" width="120" alt="codex-buddy" />
</p>

<h1 align="center">codex-buddy</h1>

<p align="center">
  一个<b>精简、快</b>的工具，让多个 <a href="https://developers.openai.com/codex">Codex CLI</a> 账号真正并行运行——<br/>
  单个 <b>544 KB</b> 二进制，切换或同时跑，全程不触发重新登录，默认纯本地。
</p>

<p align="center">
  <img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT" />
  <img src="https://img.shields.io/badge/rust-1.89%2B-orange.svg" alt="Rust" />
  <img src="https://img.shields.io/badge/platform-macOS-lightgrey.svg" alt="Platform" />
  <img src="https://img.shields.io/badge/binary-544K-brightgreen.svg" alt="Binary" />
</p>

<p align="center">
  <a href="README.md">English</a> | <b>简体中文</b> | <a href="README.es.md">Español</a>
</p>

## 特性

- **精简、快** —— 单个 544 KB 二进制，仅 4 个直接依赖，零 async / 零 HTTP / 零 crypto。切换账号是
  原子 `rename`，**瞬时完成**；探测哪些账号在并行运行走原生系统调用，约 **2 ms**。release 二进制经
  `opt-level=z` + `lto` + `strip` 极致压缩。
- **真·并行多账号** —— 多个账号的 Codex 会话可以真正**同时**运行，各自独立、互不干扰。
- **从不触发重新登录** —— 随便来回切换，不会被强制登出，也不会触发反滥用检测。
- **默认纯本地** —— 无遥测，CLI 零网络代码。实时用量是显式 opt-in 的，而且即便开启，请求也由
  codex 自己发出——见[实时用量](#实时用量可选)。菜单栏 App 自身唯一的网络调用是你手动触发的
  检查更新。
- **设计即安全** —— 初始化前会先备份你现有的登录，任何一步失败都自动回滚；一条 `doctor` 命令
  就能看出哪里配置不对。
- **配置共享、登录隔离** —— `config.toml` 和规则对所有账号生效；凭证从不在账号间泄漏。

## 菜单栏 App

除了 CLI，codex-buddy 还带一个原生的 macOS 菜单栏 App：点开就是一个面板，可视化展示每个账号的
用量、当前激活的是谁、哪些正在并行运行，点一下即可切换。**同样精简**——单架构 App 包不到 1 MB。

<p align="center">
  <img src="docs/panel-light.png" width="380" alt="面板（浅色）" />
  <img src="docs/panel-dark.png" width="380" alt="面板（深色）" />
</p>

- **用量环** —— codex 报告的每个限流窗口各还剩多少额度，一眼看清，按阈值着色。
- **实时用量（opt-in）** —— 点亮标题栏的闪电图标后，面板打开时会通过 codex 拉取每个账号的最新
  数字（带节流、逐账号并行）；本地数据始终是兜底。默认关闭。
- **账号列表** —— 专属糖果色头像、plan 徽章、并行运行绿点、当前账号打勾。
- **内置 Doctor** —— 面板里直接自检；有问题才展开清单，一键复制报告。
- **明暗主题** —— 跟随系统，也能手动切浅色 / 深色。
- **行内操作 + 添加账号** —— 每个账号行一排图标即可改名、复制 `CODEX_HOME`、在终端里运行、删除；
  「Add Account」原地展开，走真实 `codex login` 或从已有 `auth.json` 导入。

<p align="center">
  <img src="docs/actions.png" width="380" alt="行内操作与添加账号" />
</p>

- **菜单栏状态项** —— 不点开也能看到当前账号 + 更紧张的那个用量百分比，按阈值着色。

<p align="center">
  <img src="docs/menubar.png" width="220" alt="菜单栏状态项" />
</p>

App 下载见 [Releases](https://github.com/CodePrometheus/codex-buddy/releases)：Apple Silicon 用
`Codex-Buddy-arm64-macOS.zip`，Intel 用 `Codex-Buddy-x86_64-macOS.zip`。未签名，首次打开需右键
「打开」一次。

## 安装

**Homebrew。**

```sh
brew install CodePrometheus/tap/codex-buddy
```

**Shell 脚本。** 直接下载预编译二进制，不需要 Homebrew：

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/CodePrometheus/codex-buddy/releases/latest/download/codex-buddy-installer.sh | sh
```

两种方式都需要 Apple Silicon 或 Intel macOS；预编译二进制和校验和见 [Releases](https://github.com/CodePrometheus/codex-buddy/releases)。

## 快速上手

```
$ codex-buddy init
Detected current account:
  email : alice@work.example
  plan  : plus

Alias for this account [work]:
...
Done: account 'work' is managed and set as current.

$ codex-buddy add personal
Opening codex login for 'personal'; complete the login in your browser...
...
Account 'personal' added. Use `codex-buddy switch personal`, or `codex-buddy run personal -- ...`
to run it in parallel.

$ codex-buddy list
  ALIAS     EMAIL                   PLAN  1W        LAST USED
* work      alice@work.example      plus  12% (4d)  just now
  personal  alice@personal.example  pro   0% (6d)   2d ago

$ codex-buddy switch personal
Switched to: personal  alice@personal.example  [pro]

$ codex
# 直接进入，不会弹出登录

$ codex-buddy switch -
Switched to: work  alice@work.example  [plus]
```

两个终端同时跑两个账号，不需要切换任何一个：

```
# 终端 1
$ codex-buddy run work -- codex

# 终端 2
$ codex-buddy run personal -- codex
```

## 命令

**初始设置**

| 命令 | 说明 |
|---|---|
| `init [alias] [--yes]` | 纳管当前 `~/.codex` 账号 |
| `add <alias>` | 登录并纳入一个新账号 |
| `import <auth.json> [--alias a] [--json]` | 导入单个账号 |
| `import <directory> [--skip-existing] [--json]` | 批量导入一级 `<alias>/auth.json` 子目录；成功项保留 |
| `export <alias> <path> [--force]` | 以 `0600` 权限导出单个凭证文件 |
| `relogin <alias>` | 重新登录某个已有账号（例如 token 过期后） |
| `rename <old> <new>` | 重命名账号 |
| `remove <alias> [--yes]` | 删除账号（拒绝删除当前激活账号） |

**日常使用**

| 命令 | 说明 |
|---|---|
| `list [--json]` | 列出账号及用量 |
| `current [--json]` | 显示当前激活账号 |
| `usage [alias] [--remote] [--json]` | 查看用量及其新鲜度（fresh / expired / missing） |
| `recommend [--remote] [--json]` | 推荐额度余量最大的账号 |
| `switch <alias> \| - \| --next` | 切换账号（`-` 上一个，`--next` 按注册顺序轮换） |
| `run <alias> -- <args>` | 在某个账号下并行运行 codex |
| `path <alias>` | 打印某账号的 `CODEX_HOME` |
| `doctor [--json]` | 检查安装健康状态 |
| `report [--json]` | 汇总账号与健康检查 |

用量表格的列来自数据里实际存在的限流窗口——codex 上游改过窗口集合（5h 窗口就消失过），所以
这里不写死任何一列。

目录导入只扫描一级子目录：

```
accounts/
├── work/auth.json
└── personal/auth.json
```

每个账号独立提交。命令会为每一项打印 `imported`、`skipped` 或 `failed`，保留已成功的导入，任何
一项失败则以非零码退出。`--skip-existing` 只跳过「别名相同且账号身份也相同」的已有账号，绝不
替换凭证。

导出的 `auth.json` 含 access / refresh token。codex-buddy 会以 `0600` 权限创建它，拒绝 symlink
和受管的 `~/.codex-buddy` 目标路径，覆盖已有普通文件必须显式 `--force`。

Codex 需要把登录信息存成普通文件，而不是存进系统钥匙串——codex-buddy 要直接管理这份文件，所以
它必须在磁盘上。`init`、`add` 会自动检查，不满足时会告诉你怎么改（在 `~/.codex/config.toml` 里设
`cli_auth_credentials_store = "file"`）。

## 实时用量（可选）

默认情况下，codex-buddy 展示的所有数字都来自本地会话数据：截至该账号上次运行 codex 时是准确的，
并如实标注 `fresh` / `expired` / `missing`。想要当前的实时数字时：

```
$ codex-buddy usage --remote
  ALIAS     STATUS  1W
  work      fresh   15% (5d)
  personal  fresh   6% (4d)

$ codex-buddy recommend --remote
Recommended: personal
  bottleneck: 1w with 94% remaining
  1w: 6% used, 94% remaining, resets in 4d
```

`--remote` 的取数方式是让 codex 自己回答：codex-buddy 以各账号目录为 `CODEX_HOME` 启动
`codex app-server`，通过其 stdio 协议读取该账号的限流数据，然后关掉它。网络请求由官方 codex
二进制发出——它自己的客户端、自己的鉴权、自己的 token 刷新。codex-buddy 永远不直连任何后端、
不含任何 HTTP 代码、不向任何地方发送数据；开了 `--remote` 也一样。

菜单栏 App 里同样的开关在标题栏的闪电图标后面（「Live usage via codex」），默认关闭，开启后
会记住你的选择。

## 许可证

[MIT License](LICENSE)
