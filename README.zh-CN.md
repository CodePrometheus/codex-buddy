# codex-buddy

[English](README.md) | **简体中文** | [Español](README.es.md)

![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.89%2B-orange.svg)
![Platform](https://img.shields.io/badge/platform-macOS-lightgrey.svg)

让多个 [Codex CLI](https://developers.openai.com/codex) 账号真正并行运行——切换或同时跑，全程不会触发重新登录。

## 特性

- **真·并行多账号** —— 多个账号的 Codex 会话可以真正同时运行
- **从不触发重新登录** —— 随便来回切换，不会被强制登出，也不会触发反滥用检测
- **完全本地** —— 无遥测、无云依赖，数据不出本机；单个二进制体积小于 1&nbsp;MB
- **设计即安全** —— 初始化前会先备份你现有的登录，任何一步失败都自动回滚；一条 `doctor` 命令
  就能看出哪里配置不对
- **配置共享、登录隔离** —— `config.toml` 和规则对所有账号生效；凭证从不在账号间泄漏

## 安装

**Homebrew。** tap 就是本仓库本身(不是 `homebrew-` 前缀命名),第一次需要显式 tap URL 并信任一次:

```sh
brew tap CodePrometheus/codex-buddy https://github.com/CodePrometheus/codex-buddy
brew trust codeprometheus/codex-buddy
brew install codex-buddy
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
