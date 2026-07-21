# codex-buddy

A macOS CLI to switch between and run Codex CLI accounts in parallel. Logic lives in
`crates/core`; `crates/cli` is a thin shell. `apps/tray` is a SwiftUI menu-bar app (work in
progress) that reuses `core` through `crates/ffi`'s uniffi bindings — same invariants, same
source of truth, no logic duplicated into Swift.

## How it works (the invariant everything rests on)

Each account keeps one real `auth.json` in `~/.codex-buddy/<alias>/`. `~/.codex/auth.json` is a
symlink to the active account's file. **Switching only repoints that symlink; auth files are never
copied.** This sidesteps OAuth refresh-token rotation: a credential exists as exactly one file,
refreshed in place, so there is never a stale copy that would force a re-login.

- Switched (per account; `~/.codex/<entry>` symlinks to the active account's copy): `auth.json`,
  `sessions/`, `history.jsonl`.
- Isolated (per account, never shared or linked): all sqlite (`sqlite/` dir, `*.sqlite*`).
- Shared (symlinked back to `~/.codex`): everything else — config.toml, AGENTS.md, rules, ...
- Parallel = run codex with `CODEX_HOME=<account dir>`; switch = repoint the switched symlinks.

Hard requirement: codex's `cli_auth_credentials_store` must be `file` (keyring / auto / ephemeral
move or delete auth.json and break the scheme). `config_check` enforces this.

## Layout

- `crates/core` — all logic, no CLI or interactive IO. Modules: `paths`, `error`, `auth`,
  `registry`, `layout`, `config_check`, `init`, `ops`, `doctor`, `usage`, `running`. Unit tests
  live in `src/<module>/tests.rs`.
- `crates/cli` — arg parsing (pico-args), prompts, output; delegates everything to core.
- `crates/ffi` — uniffi bindings over `core` for the Swift tray (`list_accounts`, `switchAccount`,
  `addAccount`, ...). Thin: no business logic, just type conversion. `core` and `cli` stay free of
  any FFI dependency.
- `crates/ffi-bindgen` — a separate `uniffi-bindgen` binary crate that generates the Swift
  bindings from the built `codex-buddy-ffi` library. Split out so its `uniffi/cli` feature (pulls
  in `clap`) never ends up in the shipped cdylib's dependency graph.
- `apps/tray` — the SwiftUI menu-bar app (Swift Package, not part of the Cargo workspace). Run
  `apps/tray/Scripts/build-ffi.sh` first to build the xcframework + generate the Swift bindings
  (both gitignored, regenerated from `crates/ffi`), then `swift build` inside `apps/tray`.

## Conventions

- Code, comments, and strings are English. Comments stay minimal: a short `///` on structs,
  fields, and important methods; only "why" notes in bodies; no module-level `//!` (except the
  one crate doc in `lib.rs`); no comments in `Cargo.toml` / `rust-toolchain.toml`. Test functions
  get no doc unless there is something notable.
- Dependencies stay tiny: `serde`, `serde_json`, `base64`, `pico-args`. No tokio / async / HTTP /
  crypto / chrono. JWTs are decoded, never verified. The release profile is size-optimized. The
  one exception is `uniffi` in `crates/ffi`/`crates/ffi-bindgen` — required to bridge to Swift;
  `core` and `cli` are still zero-FFI and unaffected.
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
