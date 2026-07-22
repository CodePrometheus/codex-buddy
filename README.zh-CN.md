# codex-buddy

[English](README.md) | **简体中文** | [Español](README.es.md)

![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.89%2B-orange.svg)
![Platform](https://img.shields.io/badge/platform-macOS-lightgrey.svg)
![Binary](https://img.shields.io/badge/binary-461K-brightgreen.svg)

一个**精简、快**的工具,让多个 [Codex CLI](https://developers.openai.com/codex) 账号真正并行运行——单个 **461 KB** 二进制,切换或同时跑,全程不触发重新登录,数据不出本机。

## 特性

- **精简、快** —— 单个 461 KB 二进制,仅 4 个直接依赖,零 async / 零 HTTP / 零 crypto。切换账号是
  原子 `rename`,**瞬时完成**;探测哪些账号在并行运行走原生系统调用,约 **2 ms**。release 二进制经
  `opt-level=z` + `lto` + `strip` 极致压缩。
- **真·并行多账号** —— 多个账号的 Codex 会话可以真正**同时**运行,各自独立、互不干扰。
- **从不触发重新登录** —— 随便来回切换,不会被强制登出,也不会触发反滥用检测。
- **完全本地** —— 无遥测、无云依赖、无任何网络调用,数据不出本机。
- **设计即安全** —— 初始化前会先备份你现有的登录,任何一步失败都自动回滚;一条 `doctor` 命令
  就能看出哪里配置不对。
- **配置共享、登录隔离** —— `config.toml` 和规则对所有账号生效;凭证从不在账号间泄漏。

## 菜单栏 App

除了 CLI,codex-buddy 还带一个原生的 macOS 菜单栏 App:点开就是一个面板,可视化展示每个账号的
用量、当前激活的是谁、哪些正在并行运行,点一下即可切换。**同样精简**——单架构 App 包只有 **0.6 MB**。

<p align="center">
  <img src="docs/panel-light.png" width="380" alt="面板(浅色)" />
  <img src="docs/panel-dark.png" width="380" alt="面板(深色)" />
</p>

- **双环用量** —— 一眼看清 5h / 7d 两个窗口各还剩多少额度,按阈值着色。
- **账号列表** —— 专属糖果色头像、plan 徽章、并行运行绿点、当前账号打勾。
- **内置 Doctor** —— 面板里直接自检;有问题才展开清单,一键复制报告。
- **明暗主题** —— 跟随系统,也能手动切浅色 / 深色(上图即浅 / 深两版)。
- **行内操作 + 添加账号** —— 每个账号行一排图标即可改名、复制 `CODEX_HOME`、在终端里运行、删除;
  「Add Account」原地展开,走真实 `codex login` 或从已有 `auth.json` 导入。

<p align="center">
  <img src="docs/actions.png" width="380" alt="行内操作与添加账号" />
</p>

- **菜单栏状态项** —— 不点开也能看到当前账号 + 更紧张的那个用量百分比,按阈值着色。

<p align="center">
  <img src="docs/menubar.png" width="220" alt="菜单栏状态项" />
</p>

App 下载见 [Releases](https://github.com/CodePrometheus/codex-buddy/releases):Apple Silicon 用
`Codex-Buddy-arm64-macOS.zip`,Intel 用 `Codex-Buddy-x86_64-macOS.zip`。未签名,首次打开需右键
「打开」一次。

## 安装

**Homebrew。**

```sh
brew install CodePrometheus/tap/codex-buddy
```

**Shell 脚本。** 直接下载预编译二进制,不需要 Homebrew:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/CodePrometheus/codex-buddy/releases/latest/download/codex-buddy-installer.sh | sh
```

两种方式都需要 Apple Silicon 或 Intel macOS;预编译二进制和校验和见 [Releases](https://github.com/CodePrometheus/codex-buddy/releases)。

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
  ALIAS      EMAIL                  PLAN  5H  1W       ACTIVE
* work       alice@work.example     plus  -   12% (4d)  just now
  personal   alice@personal.example pro   -   0% (6d)   2d ago

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
| `import <path> [--alias a]` | 从已有的 `auth.json` 纳入账号 |
| `relogin <alias>` | 重新登录某个已有账号（例如 token 过期后） |
| `rename <old> <new>` | 重命名账号 |
| `remove <alias> [--yes]` | 删除账号（拒绝删除当前激活账号） |

**日常使用**

| 命令 | 说明 |
|---|---|
| `list` | 列出账号及用量 |
| `current` | 显示当前激活账号 |
| `switch <alias> \| -` | 切换账号（`-` 表示上一个） |
| `run <alias> -- <args>` | 在某个账号下并行运行 codex |
| `path <alias>` | 打印某账号的 `CODEX_HOME` |
| `doctor` | 检查安装健康状态 |

Codex 需要把登录信息存成普通文件，而不是存进系统钥匙串——codex-buddy 要直接管理这份文件，所以
它必须在磁盘上。`init`、`add` 会自动检查，不满足时会告诉你怎么改（在 `~/.codex/config.toml` 里设
`cli_auth_credentials_store = "file"`）。

## 许可证

[MIT License](LICENSE)
