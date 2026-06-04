# AGENTS.md

Non-negotiable rules for this repository. Any violation is a defect, not a preference.

## Naming

- **NEVER abbreviate** — identifiers, functions, directories, files use full words.
  `index` not `i`, `sources` not `src`, `library` not `lib`, `configuration` not `config`.
- Only exception: names a tool hard-requires (`Cargo.toml`, `.cargo/config.toml`,
  `rust-toolchain.toml`, `tsconfig.json`, `vite.config.ts`, `deny.toml`).

## Code

- **NEVER write comments.** No `//`, `///`, `//!`, `/* */`, JSDoc. Explanation lives in Markdown only.
- **NEVER use panic-prone or unsafe Rust** in non-test code: no `unwrap()`, `expect()`, `panic!`,
  `unreachable!`, `unsafe`, or indexing/slicing/arithmetic that can panic. Propagate errors with
  `Result`/`Option`, handle every case, and use saturating/checked arithmetic.
- **Indent with 2 spaces everywhere**, including Rust (do not use the 4-space default).
- **Minimal dependencies.** Add an external crate/package only if it is essential and large.
  If it can be written compactly in-house, put it under `crates/` instead.
- **Every file and line must justify its existence.** If you cannot explain why it is there, remove it.

## Layout (flat root)

- `packages/` — desktop applications (each has Rust shell + Solid/TypeScript frontend).
  - `cursor-optimizer/` — main app: Rust shell (`sources/`) + frontend (`sources/web/`).
  - `cursor-optimizer-installer/` — graphical installer: Rust shell + frontend.
- `libraries/` — shared frontend packages consumed by both apps.
  - `design-system/` — CSS tokens, base styles, animations.
  - `user-interface/` — shared components (TitleBar, FileBrowserField, buttons), IPC, types.
- `crates/` — GUI-agnostic Rust libraries written in-house.
- `distributions/` — all build output (Cargo target dir + web bundle). Git-ignored.
- `artifacts/` — final shippable binaries only.
- Source code always lives in a `sources/` directory. Never create `src`.

## Stack (do not swap without explicit approval)

- Backend: **wry + tao** (system webview). NEVER Tauri, Electron, or a bundled browser engine.
- Frontend: **Solid + Vite + TypeScript**.
- Database: **rusqlite** (bundled SQLite).
- Rust: **latest stable**, pinned in `rust-toolchain.toml`; upgrade the toolchain if it is older.

## Self-contained binary

- Bundle every asset (fonts, CSS, icons) at build time and serve from embedded bytes.
  **NEVER fetch from the network at runtime.**
- The webview data directory MUST live in OS app-data, never beside the executable.

## UX & design — match cursor.com

- Light, warm, calm, high-density. Canvas `#f7f7f4`, ink `#26251e`, single accent `#f54e00`,
  pill buttons, Pretendard font.
- **NEVER expose internal/developer text** (raw keys, SQL, error internals, log spam). Use friendly labels.
- Make long operations visible (live scan/progress). Give immediate visual feedback
  (changing an input instantly previews its effect, e.g. space to be freed). Keep views minimal.
- Insight/reads are NEVER blocked. Block only writes while Cursor runs, and offer force-quit.

## Quality gates (must always pass, zero warnings)

- **Rust**: `cargo clippy --all-targets --workspace -- --deny warnings` must be clean.
  The workspace forbids `unsafe`, denies all clippy + rustc warnings, and denies
  `unwrap`/`expect`/`panic`. `cargo fmt --all --check` must pass (2-space, `reorder_imports`).
- **TypeScript**: Biome is the linter and formatter, configured as strict as practical.
  `biome ci` must pass with zero findings, and `tsgo --noEmit` must be clean.
  - **NEVER** use `any`.
  - **NEVER** use `as` casts, except at a genuinely-unknown external boundary
    (`JSON.parse` output, raw IPC payloads) — and even there, narrow immediately.
  - **NEVER** use non-null assertions (`!`); unwrap with `Show`/guards instead.
- **Imports are ordered** automatically: rustfmt `reorder_imports` for Rust, Biome
  `organizeImports` for TypeScript.
- **Tests are required for every area**: Rust logic has `cargo test` unit tests; frontend
  logic has Vitest tests. Both must pass.

## Build, license, CI

- Produce artifacts with **`cargo bundle`**. `cargo build` alone never populates `artifacts/`
  (Cargo has no post-build hook).
- Support Windows / macOS / Linux on x86_64 and arm64; CI builds natively per architecture.
- License: **MIT, © Jay Lee <jay@vendit.co.kr>**. Keep `LICENSE`, `THIRD-PARTY-NOTICES.md`,
  and `deny.toml` accurate; honor every bundled asset's license (OFL/MIT attribution).
  `cargo-deny` must pass.
- Keep `.gitignore`, `.gitattributes`, `.editorconfig` present and correct.
