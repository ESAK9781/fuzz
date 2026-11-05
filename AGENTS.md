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
- `embed.rs` — `Embedder` trait; `HashEmbedder` offline fallback ("keyword mode"). Unicode-aware: NFKD fold +
  strip accents, CJK as char uni/bi-grams. Text extraction decodes BOM/UTF-16/Windows-1252 and NFKC-normalises.
- `model.rs` — in-app installer for the EmbeddingGemma ONNX model (public `onnx-community/embeddinggemma-300m-ONNX`,
  quantized, ~330 MB, size-verified, emits `task-progress`). The model loads in a background thread at startup
  (`AppState.loading`); `sync_library`/`search` refuse to run with the placeholder embedder while it loads.
- `onnx.rs` (feature `onnx`, default) — EmbeddingGemma via `ort`; tries CUDA → DirectML (Windows) / CoreML
  (macOS) / CUDA (Linux) then CPU. Truncates to 256 dims (MRL) and re-normalises.
- `search.rs` — filter-first, rayon-parallel dot product over normalised vectors, parent hydration.
- `archive.rs` — single-file `.fuzz` export/import (ZIP container + `fuzz-manifest.json`).
- `tests.rs` — unit/integration tests (`cargo test --lib`).
- `ui/` — `index.html`, `style.css`, `app.js` (no bundler, `withGlobalTauri`). All user/file text is escaped
  with `esc()` before `innerHTML`.
- `ui/pdfview.js` — ES module (lazy `import()`): locates a hit's text on its PDF page via pdf.js text content,
  renders highlighted context thumbnails, and the scrollable viewer that opens scrolled to the match.
  PDF bytes come from the `read_pdf` command (binary `ipc::Response`). `ui/vendor/` is a checked-in copy of
  pdf.js 4.10.38 (+cmaps, standard fonts) renamed `.mjs` → `.js`; update by re-copying from `pdfjs-dist/build`.
- `ui/mediaview.js` — lazy ES module: viewer for audio/video/images. Files stream through Tauri's **asset
  protocol** (`protocol-asset` feature, `assetProtocol` enabled in `tauri.conf.json`); each opened library's folder
  is added to the scope at runtime in `install()`. Opens at the hit's `start_sec`, shows the indexed transcript
  (`transcript` command = timed child chunks) with the matching cue marked; clicking a cue seeks. The PDF viewer
  overlays a selectable pdf.js `TextLayer` (styles in `ui/vendor/pdf_viewer.css`).
- `src-tauri/src/msvc_shim.rs` — Windows-only shim for a vectorised-STL symbol ONNX Runtime needs when the
  installed MSVC is older than 14.44.

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
- `ureq` is pinned to `~3.1` (newer needs a newer rustc). Real-model check (downloads ~330 MB into the config dir):
  `cargo test --lib real_model -- --ignored --nocapture`.
- **Driving the real UI for verification without touching the desktop:** launch the debug exe with
  `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` and talk CDP to `127.0.0.1:9222`
  (evaluate JS, `Page.captureScreenshot`). Don't use SendKeys/full-screen captures: they hit other windows.
  Debug builds open a console window; release builds don't. Never kill `fuzz.exe` blindly: the user may have it open.
- The semantic model is not bundled; the app offers a one-click install (banner / Settings / pill). Manual
  install: put `model_quantized.onnx(+_data)` and `tokenizer.json` in the model folder. Without it: keyword mode.
  Match-meter calibration (`meterHtml` in `app.js`) maps cosine to 0–100 per engine; revisit if the model changes.

## Known limitations / roadmap
- Audio/video/images are searchable via sidecar text only (`name.srt|vtt|transcript.txt`, images
  `name.caption.txt`) plus filename; no built-in transcription/OCR/captioning yet.
- Video/audio play in the WebView2 player: MP4(H.264)/WebM/MP3/WAV/M4A/OGG work, but MKV/AVI/FLAC/WMV may not;
  the viewer then offers "Open in default app". Video thumbnails are the browser's frame at the timestamp.
- Still missing: built-in transcription (audio/video are searchable only via sidecar transcripts), OCR/captioning
  for images, in-PDF find, and a PDF page sidebar.
- The quantized int8 model runs on CPU even when a GPU provider is listed (many int8 ops lack GPU kernels); an
  fp16/fp32 model would use CUDA/DirectML more fully.
- **Releases are not set up yet but are planned** (GitHub Actions + `tauri-action` producing .exe/.msi,
  .dmg, .deb/.AppImage, with signing). Don't add release workflows until asked.
