# fuzz

Search your own files **by meaning**, fully offline. fuzz is a small desktop app (Tauri v2 + Rust) that
indexes a folder of PDFs, notes, recordings, video and images into one portable JSON index and finds the
exact quote, passage or document you're thinking of.

## Features
- **Local & private** — embeddings run on your machine (EmbeddingGemma via ONNX Runtime, using CUDA / DirectML / CoreML when available, CPU otherwise). Nothing is uploaded.
- **Three result scopes** — exact quotes/sentences, whole passages, or full documents; matches show their parent context, page or timestamp.
- **Libraries are just folders** with a hidden `.semantic_index.json`. Organise media into real **subfolders**, **bookmark** items and add your own **tags**, then filter by folder, tag and media type.
- **Portable** — libraries open on Windows, macOS and Linux alike, and can be **exported to a single `.fuzz` file** and imported elsewhere.
- **Responsive** — indexing runs on background threads with live progress; unchanged files are never re-processed and renames are detected by hash.
- Works out of the box in a keyword-matching mode; add the model for true semantic search (see below).

## Run it
```bash
npm install
npm run dev        # development
npm run build      # native bundle (.exe/.msi, .dmg, .deb/.AppImage)
```
Requires [Rust](https://rustup.rs), Node 18+, and on Windows the WebView2 runtime.

### Enable semantic search
Download an ONNX export of EmbeddingGemma (it's gated on Hugging Face) and put `model.onnx` and
`tokenizer.json` in the folder shown in **Settings → Open model folder**, then press **Reload model**.

### Audio, video & images
Add a transcript or caption next to the file and it becomes searchable with timestamps:
`talk.mp3` + `talk.srt` / `talk.vtt` / `talk.transcript.txt`, `photo.png` + `photo.caption.txt`.
Built-in transcription/OCR is on the roadmap.

## Releases
Prebuilt installers are **planned** (CI-built, signed bundles for each platform) but not published yet.

## Development
See [AGENTS.md](AGENTS.md) for architecture, data-format rules and workflow.
