# codex-buddy

**English** | [简体中文](README.zh-CN.md) | [Español](README.es.md)

![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-1.89%2B-orange.svg)
![Platform](https://img.shields.io/badge/platform-macOS-lightgrey.svg)

Switch between multiple [Codex CLI](https://developers.openai.com/codex) accounts and run them
in parallel — no re-logins, ever.

## Features

- **Never triggers a re-login** — switch back and forth as much as you want; no forced logout, no
  risk of tripping anti-abuse detection
- **True parallel accounts** — run several Codex sessions under different accounts at the same
  time
- **100% local** — no telemetry, no cloud dependency, nothing leaves your machine; a single binary
  under 1&nbsp;MB
- **Safe by design** — setup backs up your existing login before touching it and rolls back on
  any failure; a one-command `doctor` check tells you if anything's off
- **Shared config, isolated logins** — `config.toml` and rules apply to every account; credentials
  never leak between them

## Quick start

```
$ codex-buddy init
Detected account: alice@work.example (plan: plus)
Alias for this account? [work]:
Adopted `work` as the active account.

$ codex-buddy add personal
Opening `codex login` for the new account...
Account `personal` added.

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
