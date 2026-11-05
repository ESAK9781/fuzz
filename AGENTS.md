# AGENTS.md — fuzz

Guide for AI coding agents (and humans) working in this repo. **Keep this file current**: when you change
architecture, commands, formats or workflow, update the relevant section in the same change.

## What fuzz is
A local-first desktop app (Tauri v2: Rust backend + plain HTML/CSS/JS frontend) that indexes a folder of
PDFs, text, audio, video and images and lets you search it by meaning. Everything stays on the device.

## Layout
- `src-tauri/src/lib.rs` — app state + all Tauri commands (`#[tauri::command]`), registered in `run()`.
- `config.rs` — `app_config.json` in the OS config dir (`directories`), recents (max 10, LRU), model dir.
- `library.rs` — `.semantic_index.json` schema (`Library`, `FileEntry`, `Chunk`), `Loaded` (flat normalised
  `Vec<f32>` vector buffer), media-type table, path sanitising (`clean_rel`).
- `extract.rs` — text/PDF/SRT/VTT → parent passages + child sentences (+ page / timestamps).
- `indexer.rs` — `sync()`: scan → reuse unchanged / detect renames by hash → **parallel** (rayon pool, N-1
  threads) extract + embed. Always runs off the UI thread (`spawn_blocking`) and emits `task-progress`.
- `embed.rs` — `Embedder` trait; `HashEmbedder` offline fallback ("keyword mode").
- `onnx.rs` (feature `onnx`, default) — EmbeddingGemma via `ort`; tries CUDA → DirectML (Windows) / CoreML
  (macOS) / CUDA (Linux) then CPU. Truncates to 256 dims (MRL) and re-normalises.
- `search.rs` — filter-first, rayon-parallel dot product over normalised vectors, parent hydration.
- `archive.rs` — single-file `.fuzz` export/import (ZIP container + `fuzz-manifest.json`).
- `tests.rs` — unit/integration tests (`cargo test --lib`).
- `ui/` — `index.html`, `style.css`, `app.js` (no bundler, `withGlobalTauri`). All user/file text is escaped
  with `esc()` before `innerHTML`.

## Data & compatibility rules (do not break)
- Library = a folder + hidden `.semantic_index.json`. Paths in the index are **relative, `/`-separated**,
  timestamps are unix seconds, hashes are content-based → a library works across Windows/macOS/Linux.
  Never store absolute paths in the index.
- `library_version` is major-checked on load (newer major → refuse with a clear message). Unknown JSON
  fields are ignored; add new fields with `#[serde(default)]`.
- If the index's `embedding_model` differs from the device's active embedder, search is paused and the UI
  shows a banner; the user explicitly re-indexes. Never silently mix vectors from different models.
- Tags are per-file (`FileEntry.tags`); `bookmark` is just a reserved tag. Subfolders are real folders.
- Export format: ZIP with `fuzz-manifest.json` + `.semantic_index.json` + all non-hidden files. Import
  uses `enclosed_name()` (zip-slip safe) and cleans up on failure.

## Build / test
- Needs Rust (MSVC on Windows), Node, WebView2 (Windows). The repo pins an MSRV-friendly dependency set:
  `src-tauri/.cargo/config.toml` enables `incompatible-rust-versions = "fallback"` and `Cargo.toml` has
  `rust-version = "1.89"`; `ort` is pinned to `=2.0.0-rc.13`. Don't bump these without checking the toolchain.
- `npm install`, then `npm run dev` (dev) / `npm run build` (bundle).
- `cd src-tauri && cargo test --lib`.
- **Run only one build/test at a time**, and prefer running them in the background while you work.
- The semantic model is NOT bundled (gated on Hugging Face). Users drop `model.onnx` + `tokenizer.json` in
  the model folder (Settings → Open model folder). Without it the app runs in keyword mode.

## Known limitations / roadmap
- Audio/video/images are searchable via sidecar text only (`name.srt|vtt|transcript.txt`, images
  `name.caption.txt`) plus filename; no built-in transcription/OCR/captioning yet.
- "Open at page/timestamp" opens the file in the default app; the location is shown, not deep-linked.
- **Releases are not set up yet but are planned** (GitHub Actions + `tauri-action` producing .exe/.msi,
  .dmg, .deb/.AppImage, with signing). Don't add release workflows until asked.
