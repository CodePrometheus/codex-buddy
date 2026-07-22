# codex-buddy

**English** | [简体中文](README.zh-CN.md) | [Español](README.es.md)

![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.89%2B-orange.svg)
![Platform](https://img.shields.io/badge/platform-macOS-lightgrey.svg)
![Binary](https://img.shields.io/badge/binary-461K-brightgreen.svg)

A **tiny, fast** way to run multiple [Codex CLI](https://developers.openai.com/codex) accounts in
parallel — one **461 KB** binary, switch or run side by side, no re-logins, nothing leaves your
machine.

## Features

- **Tiny & fast** — a single 461 KB binary, just 4 direct dependencies, zero async / zero HTTP /
  zero crypto. Switching accounts is an atomic `rename` (**instant**); probing which accounts are
  running in parallel uses a native syscall (~**2 ms**). The release binary is squeezed with
  `opt-level=z` + `lto` + `strip`.
- **True parallel accounts** — actually run two or more Codex sessions **at the same time**, each
  under its own account, fully isolated.
- **Never triggers a re-login** — switch back and forth as much as you want; no forced logout, no
  risk of tripping anti-abuse detection.
- **100% local** — no telemetry, no cloud dependency, no network calls at all; nothing leaves your
  machine.
- **Safe by design** — setup backs up your existing login before touching it and rolls back on
  any failure; a one-command `doctor` check tells you if anything's off.
- **Shared config, isolated logins** — `config.toml` and rules apply to every account; credentials
  never leak between them.

## Menu bar app

Beyond the CLI, codex-buddy ships a native macOS menu bar app: click the icon and a panel shows
each account's usage, which one is active, and which are running in parallel — one click to switch.
**Just as tiny** — a single-arch app bundle is only **0.6 MB**.

<p align="center">
  <img src="docs/panel-light.png" width="380" alt="Panel (light)" />
  <img src="docs/panel-dark.png" width="380" alt="Panel (dark)" />
</p>

- **Dual usage rings** — see at a glance how much is left in the 5h / 7d windows, color-coded by
  threshold.
- **Account list** — per-account candy-color avatar, plan badge, parallel-running green dot, and a
  checkmark on the active one.
- **Built-in Doctor** — self-check right in the panel; it expands a list only when something's off,
  with one-click copy of the report.
- **Light / dark** — follows the system, or toggle light / dark yourself (the two panels above).
- **Inline actions + Add account** — a row of icons per account to rename, copy `CODEX_HOME`, run
  in Terminal, or remove; "Add Account" expands in place, driving a real `codex login` or importing
  an existing `auth.json`.

<p align="center">
  <img src="docs/actions.png" width="380" alt="Inline actions and add account" />
</p>

- **Menu bar status item** — see the active account and its tightest usage percentage without even
  opening the panel, color-coded by threshold.

<p align="center">
  <img src="docs/menubar.png" width="220" alt="Menu bar status item" />
</p>

Download the app from [Releases](https://github.com/CodePrometheus/codex-buddy/releases):
`Codex-Buddy-arm64-macOS.zip` for Apple Silicon, `Codex-Buddy-x86_64-macOS.zip` for Intel. It's
unsigned, so the first launch needs a right-click → Open.

## Install

**Homebrew.**

```sh
brew install CodePrometheus/tap/codex-buddy
```

**Shell script.** Downloads a prebuilt binary, no Homebrew needed:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/CodePrometheus/codex-buddy/releases/latest/download/codex-buddy-installer.sh | sh
```

Both need Apple Silicon or Intel macOS; see [Releases](https://github.com/CodePrometheus/codex-buddy/releases) for prebuilt binaries and checksums.

## Quick start

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
# starts immediately, no login prompt

$ codex-buddy switch -
Switched to: work  alice@work.example  [plus]
```

Run two accounts side by side without switching either one:

```
# terminal 1
$ codex-buddy run work -- codex

# terminal 2
$ codex-buddy run personal -- codex
```

## Commands

**Setup**

| Command | Description |
|---|---|
| `init [alias] [--yes]` | Adopt the current `~/.codex` account |
| `add <alias>` | Log in and adopt a new account |
| `import <path> [--alias a]` | Adopt an account from an existing `auth.json` |
| `relogin <alias>` | Re-login an existing account (e.g. after token expiry) |
| `rename <old> <new>` | Rename an account |
| `remove <alias> [--yes]` | Remove an account (refuses to remove the active one) |

**Use**

| Command | Description |
|---|---|
| `list` | List accounts with usage |
| `current` | Show the active account |
| `switch <alias> \| -` | Switch account (`-` = previous) |
| `run <alias> -- <args>` | Run codex under an account, in parallel |
| `path <alias>` | Print an account's `CODEX_HOME` |
| `doctor` | Check setup health |

Codex must be storing your login as a plain file, not in the system keychain — codex-buddy
manages that file directly, so it needs it on disk. `init` and `add` check this automatically and
tell you how to fix it (`cli_auth_credentials_store = "file"` in `~/.codex/config.toml`) if not.

## License

[MIT License](LICENSE)
