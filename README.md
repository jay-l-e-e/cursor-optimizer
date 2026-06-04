<p align="center">
  <img src="assets/icon.svg" width="96" height="96" alt="Cursor Optimizer">
</p>

<h1 align="center">Cursor Optimizer</h1>

<p align="center">
  Reclaim gigabytes from <a href="https://cursor.com">Cursor</a>, instantly.<br>
  Single binary, system webview, zero network calls.
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-f54e00?style=flat-square" alt="MIT License"></a>
  <img src="https://img.shields.io/badge/rust-stable-f54e00?style=flat-square&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/windows%20%C2%B7%20mac%20%C2%B7%20linux-f54e00?style=flat-square&label=platforms" alt="Platforms">
  <img src="https://img.shields.io/badge/x64%20%C2%B7%20arm64-f54e00?style=flat-square&label=arch" alt="Architectures">
</p>

<br>
<table>
<tr>
<td width="140"><br><kbd>&nbsp;Overview&nbsp;</kbd><br><br></td>
<td><b>Storage at a glance.</b><br>Visual breakdown of every stored category.</td>
</tr>
<tr>
<td><br><kbd>&nbsp;Light&nbsp;clean&nbsp;</kbd><br><br></td>
<td><b>Safe compaction, zero chat deletion.</b><br>Flushes pending writes and compacts storage without deleting chat payloads.</td>
</tr>
<tr>
<td><br><kbd>&nbsp;Deep&nbsp;clean&nbsp;</kbd><br><br></td>
<td><b>Time-based purge.</b><br>Pick a retention window: <kbd>30</kbd> / <kbd>60</kbd> / <kbd>90</kbd> / <kbd>180</kbd> days or <kbd>custom</kbd>. Preview exactly how much will be freed, then permanently delete old conversations in one click. When no old conversations match but freelist pages exist, runs compaction only.</td>
</tr>
<tr>
<td><br><kbd>&nbsp;Flush database&nbsp;</kbd><br><br></td>
<td><b>Destructive database flush.</b><br>Applies Cursor's cleanup recipe for oversized `state.vscdb` files. Existing chats may no longer open.</td>
</tr>
<tr>
<td><br><kbd>&nbsp;Tools&nbsp;</kbd><br><br></td>
<td><b>Power utilities.</b><br><kbd>Create backup</kbd> (compressed, with selectable level) · <kbd>Integrity check</kbd> · <kbd>Flush pending writes</kbd> · <kbd>Compact file</kbd> · <kbd>Refresh statistics</kbd><br>One card per action, confirm-and-go. No terminal needed.</td>
</tr>
</table>

<details>
<summary>&ensp;<b>How it works</b></summary>
<br>

&emsp;Cursor stores all chat history, agent data, and workspace state in a single SQLite file:

```
Windows   %APPDATA%\Cursor\User\globalStorage\state.vscdb
macOS     ~/Library/Application Support/Cursor/User/globalStorage/state.vscdb
Linux     ~/.config/Cursor/User/globalStorage/state.vscdb
```

&emsp;The database has two tables. `cursorDiskKV` holds everything (chats, agent blobs, checkpoints);\
&emsp;`ItemTable` holds workspace item references. This app only ever deletes rows from `cursorDiskKV`.

| Operation                | What happens inside `state.vscdb`                                                                                                                                                                                      |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Light clean**          | Checkpoints pending writes and VACUUMs. It does not delete chat payload rows.                                                                                                                                          |
| **Deep clean**           | Identifies conversation rows older than your chosen retention window, deletes those conversation rows, then checkpoints and VACUUMs.                                                                                   |
| **Flush database**       | Applies Cursor's destructive flush recipe: `PRAGMA journal_mode=DELETE`, deletes `agentKv:%`, `bubbleId:%`, and `checkpointId:%` rows, then VACUUMs. Existing chats may no longer open.                                |
| **Compact**              | `VACUUM` rebuilds the file and returns freelist pages to disk.                                                                                                                                                         |
| **Flush pending writes** | `PRAGMA wal_checkpoint(TRUNCATE)` flushes the WAL into the main file and truncates it.                                                                                                                                 |
| **Integrity check**      | `PRAGMA integrity_check` (read-only, safe while Cursor is open).                                                                                                                                                       |
| **Create backup**        | Compresses `state.vscdb` with zstd (level selectable: Fast / Balanced / Maximum) and saves the `.zst` archive to your chosen location. Typically reduces the file to ~10 % of its original size. No database mutation. |

</details>

<details>
<summary>&ensp;<b>How it stays safe</b></summary>
<br>

&emsp;Reads are **never** blocked. Browse storage stats even while Cursor is open.\
&emsp;Writes are gated: the app detects a running Cursor process and disables destructive actions until you close it (or hit **Close Cursor**).\
&emsp;Interrupted writes are journaled and auto-recovered on next launch.

</details>

<br>

> 💡 **Tip: back up before destructive operations.**
>
> Both **Deep clean** and **Flush database** offer to create a compressed backup before proceeding.
> Zstd compression typically shrinks `state.vscdb` to roughly a tenth of its original size,
> so even a multi-gigabyte database produces a manageable archive.
> If anything goes wrong, you can decompress the `.zst` file with any zstd-compatible tool
> (e.g. `zstd -d <file>.zst`) and replace the original database to fully restore your data.

> ⚠️ **Deep clean & Flush database**
>
> removes conversation records from the index database. Deleted chats will still
> appear in the sidebar list but show a perpetual loading spinner when clicked. This is expected
> behavior, not a bug. The original transcript files are preserved intact at
> `<Cursor install path>/User/workspaceStorage`. If you ever need to retrieve old history,
> those `.jsonl` files are the only remaining source.

<br>

## Getting Started

```bash
npm run development
```

Launches the Rust shell with a hot-reloading Vite dev server attached.

## Build

```bash
cargo bundle
```

Compiles web assets, release binaries, and the installer into `artifacts/`.

## Quality

```bash
npm run check
```

Runs all gates in one pass: cargo fmt, clippy, cargo test, biome, tsgo, vitest. Zero warnings required.

## Linux Dependencies

```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev librsvg2-dev
```

---

<p align="center">
  <a href="LICENSE">MIT</a> &copy; Jay Lee
  &ensp;&middot;&ensp;
  <a href="THIRD-PARTY-NOTICES.md">Third-Party Notices</a>
</p>
