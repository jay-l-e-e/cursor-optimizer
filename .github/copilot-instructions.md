# Copilot Instructions

`AGENTS.md` at the repository root is the single source of truth and every rule in it is mandatory. Read it and comply. The most-violated rules:

- No abbreviations in names (`index`/`sources`/`library`; only tool-mandated filenames are exempt).
- No code comments; 2-space indentation everywhere, including Rust.
- Minimal dependencies; build small things in-house under `crates/`.
- Fixed stack: wry + tao, Solid + Vite + TypeScript, rusqlite, latest stable Rust. No Tauri/Electron.
- Embed all assets; never fetch at runtime; never expose internal/developer text.
- Artifacts via `cargo bundle`. MIT © Jay Lee <jay@vendit.co.kr>; keep `cargo-deny` passing.
