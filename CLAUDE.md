# codex-buddy

A macOS CLI to switch between and run Codex CLI accounts in parallel. Logic lives in
`crates/core`; `crates/cli` is a thin shell. A macOS menu-bar tray reusing `core` is a later phase.

## How it works (the invariant everything rests on)

Each account keeps one real `auth.json` in `~/.codex-buddy/<alias>/`. `~/.codex/auth.json` is a
symlink to the active account's file. **Switching only repoints that symlink; auth files are never
copied.** This sidesteps OAuth refresh-token rotation: a credential exists as exactly one file,
refreshed in place, so there is never a stale copy that would force a re-login.

- Isolated (per account, never shared): `auth.json`, and all sqlite (`sqlite/` dir, `*.sqlite*`).
- Shared (symlinked back to `~/.codex`): everything else — config.toml, AGENTS.md, rules,
  sessions, history.jsonl, ...
- Parallel = run codex with `CODEX_HOME=<account dir>`; switch = repoint `~/.codex/auth.json`.

Hard requirement: codex's `cli_auth_credentials_store` must be `file` (keyring / auto / ephemeral
move or delete auth.json and break the scheme). `config_check` enforces this.

## Layout

- `crates/core` — all logic, no CLI or interactive IO. Modules: `paths`, `error`, `auth`,
  `registry`, `layout`, `config_check`, `init`, `ops`. Unit tests live in `src/<module>/tests.rs`.
- `crates/cli` — arg parsing (pico-args), prompts, output; delegates everything to core.

## Conventions

- Code, comments, and strings are English. Comments stay minimal: a short `///` on structs,
  fields, and important methods; only "why" notes in bodies; no module-level `//!` (except the
  one crate doc in `lib.rs`); no comments in `Cargo.toml` / `rust-toolchain.toml`. Test functions
  get no doc unless there is something notable.
- Dependencies stay tiny: `serde`, `serde_json`, `base64`, `pico-args`. No tokio / async / HTTP /
  crypto / chrono. JWTs are decoded, never verified. The release profile is size-optimized.
- Writes are atomic: the registry via temp-file + rename under a file lock; symlink repointing via
  temp symlink + rename. `init` is the only operation that touches existing `~/.codex` data, and
  it is reversible (timestamped backup + rollback on any failure).

## Commands

```
cargo test
cargo clippy --all-targets
cargo fmt --all
cargo build --release
```

Keep test / clippy / fmt all green before finishing a change.
