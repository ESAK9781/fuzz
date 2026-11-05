# fuzz

Search your own files **by meaning**, fully offline. fuzz is a small desktop app (Tauri v2 + Rust) that
indexes a folder of PDFs, notes, recordings, video and images into one portable JSON index and finds the
exact quote, passage or document you're thinking of.

## Features
- **Local & private** — embeddings run on your machine (EmbeddingGemma via ONNX Runtime, using CUDA / DirectML / CoreML when available, CPU otherwise). Nothing is uploaded.
- **See the match in the PDF** — each PDF result shows a small thumbnail of the page with the matched text highlighted in its surrounding lines. Click it to open a viewer scrolled to that exact spot.
- **Match meter** — a circular meter on every result shows how close the match is.
- **Three result scopes** — exact quotes/sentences, whole passages, or full documents; matches show their parent context, page or timestamp.
- **Libraries are just folders** with a hidden `.semantic_index.json`. Organise media into real **subfolders**, **bookmark** items and add your own **tags**, then filter by folder, tag and media type.
- **Portable** — libraries open on Windows, macOS and Linux alike, and can be **exported to a single `.fuzz` file** and imported elsewhere.
- **Responsive** — indexing runs on background threads with live progress; unchanged files are never re-processed and renames are detected by hash.
- Works out of the box in a keyword-matching mode; install the semantic model with one click for true semantic search.
- **Unicode-friendly** — accents, ligatures and CJK text are handled; legacy text encodings are detected.

## Run it
```bash
npm install
npm run dev        # development
npm run build      # native bundle (.exe/.msi, .dmg, .deb/.AppImage)
```
Requires [Rust](https://rustup.rs), Node 18+, and on Windows the WebView2 runtime.

### Enable semantic search
Click **Install model** (banner, Settings, or the engine pill). fuzz downloads the quantized EmbeddingGemma ONNX export (~330 MB, one time) from Hugging Face and loads it on-device. After that nothing leaves your computer.

### Audio, video & images
Add a transcript or caption next to the file and it becomes searchable with timestamps:
`talk.mp3` + `talk.srt` / `talk.vtt` / `talk.transcript.txt`, `photo.png` + `photo.caption.txt`.
Built-in transcription/OCR is on the roadmap.

## Releases
Prebuilt installers are **planned** (CI-built, signed bundles for each platform) but not published yet.

## Development
See [AGENTS.md](AGENTS.md) for architecture, data-format rules and workflow.
