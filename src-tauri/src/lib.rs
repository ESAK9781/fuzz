mod archive;
mod config;
mod embed;
mod extract;
mod indexer;
mod library;
mod model;
#[cfg(all(windows, feature = "onnx"))]
mod msvc_shim;
#[cfg(feature = "onnx")]
mod onnx;
mod search;
#[cfg(test)]
mod tests;

use config::AppConfig;
use embed::Embedder;
use library::*;
use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

struct AppState {
    config: Mutex<AppConfig>,
    lib: RwLock<Option<Loaded>>,
    embedder: RwLock<Arc<dyn Embedder>>,
    /// True while the ONNX model is being loaded in the background at startup.
    loading: AtomicBool,
}

type Res<T> = Result<T, String>;

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

#[derive(Serialize)]
struct FileView {
    id: String,
    path: String,
    media_type: String,
    tags: Vec<String>,
    size: u64,
    last_modified: i64,
    has_text: bool,
}

#[derive(Serialize)]
struct LibraryView {
    name: String,
    path: String,
    embedding_model: String,
    files: Vec<FileView>,
    folders: Vec<String>,
    tags: Vec<(String, usize)>,
    /// Index vectors were produced by a different embedding model than this device uses.
    model_mismatch: bool,
}

fn view(state: &AppState, l: &Loaded) -> LibraryView {
    let active = state.embedder.read().name().to_string();
    let model_mismatch = !state.loading.load(Ordering::Relaxed) && !l.lib.chunks.is_empty() && !l.lib.embedding_model.is_empty() && l.lib.embedding_model != active;
    let mut tag_counts: BTreeMap<String, usize> = BTreeMap::new();
    let files = l
        .lib
        .files
        .iter()
        .map(|(id, f)| {
            for t in &f.tags {
                *tag_counts.entry(t.clone()).or_default() += 1;
            }
            FileView {
                id: id.clone(),
                path: f.path.clone(),
                media_type: f.media_type.clone(),
                tags: f.tags.clone(),
                size: f.size,
                last_modified: f.last_modified,
                has_text: f.has_text,
            }
        })
        .collect();
    LibraryView {
        name: l.lib.name.clone(),
        path: l.root.to_string_lossy().into_owned(),
        embedding_model: l.lib.embedding_model.clone(),
        files,
        folders: l.lib.folders.clone(),
        tags: tag_counts.into_iter().collect(),
        model_mismatch,
    }
}

#[derive(Serialize)]
struct Bootstrap {
    config: AppConfig,
    model: ModelStatus,
}

#[derive(Serialize, Clone)]
struct ModelStatus {
    active: String,
    installed: bool,
    loading: bool,
    download_bytes: u64,
    semantic: bool,
    accelerator: String,
    model_dir: String,
}

fn model_status(state: &AppState) -> ModelStatus {
    let emb = state.embedder.read();
    let name = emb.name().to_string();
    ModelStatus {
        semantic: !name.contains("hash"),
        installed: model::installed(&config::model_dir()),
        loading: state.loading.load(Ordering::Relaxed),
        download_bytes: model::total_bytes(),
        accelerator: emb.accelerator(),
        active: name,
        model_dir: config::model_dir().to_string_lossy().into_owned(),
    }
}

#[tauri::command]
fn bootstrap(state: State<AppState>) -> Bootstrap {
    Bootstrap { config: state.config.lock().clone(), model: model_status(&state) }
}

#[tauri::command]
fn set_default_directory(state: State<AppState>, path: String) -> Res<AppConfig> {
    let mut c = state.config.lock();
    c.default_library_directory = path;
    c.save();
    Ok(c.clone())
}

#[tauri::command]
fn forget_recent(state: State<AppState>, path: String) -> AppConfig {
    let mut c = state.config.lock();
    c.forget(&path);
    c.save();
    c.clone()
}

fn install(state: &AppState, root: PathBuf, lib: Library) -> LibraryView {
    let mut cfg = state.config.lock();
    cfg.touch_recent(&lib.name, &root.to_string_lossy());
    cfg.save();
    drop(cfg);
    let loaded = Loaded::new(root, lib);
    let v = view(state, &loaded);
    *state.lib.write() = Some(loaded);
    v
}

#[tauri::command]
async fn open_library(state: State<'_, AppState>, path: String) -> Res<LibraryView> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        state.config.lock().forget(&path);
        return Err(format!("Folder not found: {path}"));
    }
    let r = root.clone();
    let lib = tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<Library> {
        if r.join(INDEX_FILE).exists() {
            Library::load_from_json(&r)
        } else {
            let name = r.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or("Library".into());
            let lib = Library::empty(&name);
            lib.save_to_json(&r)?;
            Ok(lib)
        }
    })
    .await
    .map_err(err)?
    .map_err(|e| format!("Could not read {INDEX_FILE}: {e}"))?;
    Ok(install(&state, root, lib))
}

#[tauri::command]
async fn create_library(state: State<'_, AppState>, name: String) -> Res<LibraryView> {
    let clean = sanitize_name(&name);
    if clean.is_empty() {
        return Err("Please enter a library name.".into());
    }
    let base = PathBuf::from(state.config.lock().default_library_directory.clone());
    let root = base.join(&clean);
    if root.join(INDEX_FILE).exists() {
        return Err(format!("A library named “{clean}” already exists."));
    }
    std::fs::create_dir_all(&root).map_err(err)?;
    let lib = Library::empty(&clean);
    lib.save_to_json(&root).map_err(err)?;
    Ok(install(&state, root, lib))
}

#[derive(Serialize, Clone)]
struct Progress {
    label: &'static str,
    done: usize,
    total: usize,
    current: String,
}

#[derive(Serialize)]
struct SyncResult {
    library: LibraryView,
    added: usize,
    updated: usize,
    removed: usize,
}

#[tauri::command]
async fn sync_library(app: AppHandle, state: State<'_, AppState>) -> Res<SyncResult> {
    if state.loading.load(Ordering::Relaxed) {
        return Err("The semantic model is still loading. Try again in a moment.".into());
    }
    let (root, snapshot) = {
        let g = state.lib.read();
        let l = g.as_ref().ok_or("No library open")?;
        (l.root.clone(), l.lib.clone())
    };
    let emb = state.embedder.read().clone();
    let app2 = app.clone();
    let (mut new_lib, stats) = tauri::async_runtime::spawn_blocking(move || {
        indexer::sync(&root, &snapshot, emb.as_ref(), &|done, total, cur| {
            let _ = app2.emit("task-progress", Progress { label: "Indexing", done, total, current: cur.to_string() });
        })
    })
    .await
    .map_err(err)?;

    let mut g = state.lib.write();
    let l = g.as_mut().ok_or("Library was closed")?;
    // Keep tag edits made while indexing was running.
    for (id, f) in new_lib.files.iter_mut() {
        if let Some(cur) = l.lib.files.get(id) {
            f.tags = cur.tags.clone();
        }
    }
    l.lib = new_lib;
    l.rebuild();
    l.save().map_err(err)?;
    Ok(SyncResult { library: view(&state, l), added: stats.added, updated: stats.updated, removed: stats.removed })
}

#[tauri::command]
async fn search(state: State<'_, AppState>, params: search::SearchParams) -> Res<Vec<search::Hit>> {
    if state.loading.load(Ordering::Relaxed) {
        return Ok(vec![]);
    }
    let q = state.embedder.read().embed_query(&params.query);
    let g = state.lib.read();
    let l = g.as_ref().ok_or("No library open")?;
    if view_mismatch(&state, l) {
        return Ok(vec![]); // vectors from another model are not comparable; UI shows a banner
    }
    Ok(search::search(l, &q, &params))
}

fn view_mismatch(state: &AppState, l: &Loaded) -> bool {
    !state.loading.load(Ordering::Relaxed) && !l.lib.chunks.is_empty() && !l.lib.embedding_model.is_empty() && l.lib.embedding_model != state.embedder.read().name()
}

fn normalize_tag(t: &str) -> String {
    t.trim().trim_start_matches('#').to_lowercase().split_whitespace().collect::<Vec<_>>().join("-")
}

#[tauri::command]
fn set_tags(state: State<AppState>, file_ids: Vec<String>, add: Vec<String>, remove: Vec<String>) -> Res<LibraryView> {
    let mut g = state.lib.write();
    let l = g.as_mut().ok_or("No library open")?;
    let add: Vec<String> = add.iter().map(|t| normalize_tag(t)).filter(|t| !t.is_empty()).collect();
    let remove: Vec<String> = remove.iter().map(|t| normalize_tag(t)).collect();
    for id in &file_ids {
        if let Some(f) = l.lib.files.get_mut(id) {
            f.tags.retain(|t| !remove.contains(t));
            for t in &add {
                if !f.tags.contains(t) {
                    f.tags.push(t.clone());
                }
            }
            f.tags.sort();
        }
    }
    l.save().map_err(err)?;
    Ok(view(&state, l))
}

#[tauri::command]
fn create_folder(state: State<AppState>, path: String) -> Res<LibraryView> {
    let mut g = state.lib.write();
    let l = g.as_mut().ok_or("No library open")?;
    let rel = clean_rel(&path).filter(|r| !r.is_empty()).ok_or("Invalid folder name")?;
    std::fs::create_dir_all(l.root.join(&rel)).map_err(err)?;
    let mut parts = String::new();
    for seg in rel.split('/') {
        if !parts.is_empty() {
            parts.push('/');
        }
        parts.push_str(seg);
        if !l.lib.folders.contains(&parts) {
            l.lib.folders.push(parts.clone());
        }
    }
    l.lib.folders.sort();
    l.save().map_err(err)?;
    Ok(view(&state, l))
}

#[tauri::command]
fn move_files(state: State<AppState>, file_ids: Vec<String>, folder: String) -> Res<LibraryView> {
    let mut g = state.lib.write();
    let l = g.as_mut().ok_or("No library open")?;
    let dest = clean_rel(&folder).ok_or("Invalid folder")?;
    std::fs::create_dir_all(l.root.join(&dest)).map_err(err)?;
    for id in &file_ids {
        let Some(f) = l.lib.files.get(id) else { continue };
        let name = Path::new(&f.path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let new_rel = if dest.is_empty() { name.clone() } else { format!("{dest}/{name}") };
        if new_rel == f.path {
            continue;
        }
        let (from, to) = (l.root.join(&f.path), l.root.join(&new_rel));
        if to.exists() {
            return Err(format!("“{name}” already exists in that folder."));
        }
        std::fs::rename(&from, &to).map_err(err)?;
        // carry transcript / caption sidecars along
        if let (Some(stem), Some(fdir)) = (from.file_stem(), from.parent()) {
            let stem = stem.to_string_lossy().into_owned();
            for ext in ["srt", "vtt", "transcript.txt", "caption.txt"] {
                let sc = fdir.join(format!("{stem}.{ext}"));
                if sc.exists() {
                    let _ = std::fs::rename(&sc, to.with_file_name(format!("{stem}.{ext}")));
                }
            }
        }
        let f = l.lib.files.get_mut(id).unwrap();
        f.path = new_rel;
    }
    if !dest.is_empty() && !l.lib.folders.contains(&dest) {
        l.lib.folders.push(dest);
        l.lib.folders.sort();
    }
    l.save().map_err(err)?;
    Ok(view(&state, l))
}

#[tauri::command]
fn import_files(state: State<AppState>, paths: Vec<String>, folder: String) -> Res<usize> {
    let g = state.lib.read();
    let l = g.as_ref().ok_or("No library open")?;
    let dest_rel = clean_rel(&folder).ok_or("Invalid folder")?;
    let dest = l.root.join(&dest_rel);
    std::fs::create_dir_all(&dest).map_err(err)?;
    let mut n = 0;
    for p in paths {
        let src = PathBuf::from(&p);
        let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !src.is_file() || media_type_for(ext).is_none() {
            continue;
        }
        let stem = src.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let mut target = dest.join(src.file_name().unwrap());
        let mut i = 2;
        while target.exists() {
            target = dest.join(format!("{stem} ({i}).{ext}"));
            i += 1;
        }
        std::fs::copy(&src, &target).map_err(err)?;
        // bring sidecars along
        if let Some(dir) = src.parent() {
            for sx in ["srt", "vtt", "transcript.txt", "caption.txt"] {
                let sc = dir.join(format!("{stem}.{sx}"));
                if sc.exists() {
                    let t = target.file_stem().unwrap().to_string_lossy().into_owned();
                    let _ = std::fs::copy(&sc, dest.join(format!("{t}.{sx}")));
                }
            }
        }
        n += 1;
    }
    Ok(n)
}

fn abs_path(state: &AppState, file_id: &str) -> Res<PathBuf> {
    let g = state.lib.read();
    let l = g.as_ref().ok_or("No library open")?;
    let f = l.lib.files.get(file_id).ok_or("File not in library")?;
    Ok(l.root.join(&f.path))
}

fn emit_task(app: &AppHandle, label: &'static str) -> impl Fn(usize, usize, &str) + Sync {
    let app = app.clone();
    move |done, total, cur: &str| {
        let _ = app.emit("task-progress", Progress { label, done, total, current: cur.to_string() });
    }
}

/// Write the whole library (index + media) to one portable `.fuzz` file.
#[tauri::command]
async fn export_library(app: AppHandle, state: State<'_, AppState>, dest: String) -> Res<usize> {
    let (root, name) = {
        let mut g = state.lib.write();
        let l = g.as_mut().ok_or("No library open")?;
        l.save().map_err(err)?;
        (l.root.clone(), l.lib.name.clone())
    };
    let mut dest = PathBuf::from(dest);
    if dest.extension().is_none() {
        dest.set_extension("fuzz");
    }
    tauri::async_runtime::spawn_blocking(move || archive::export(&root, &name, &dest, &emit_task(&app, "Exporting")))
        .await
        .map_err(err)?
        .map_err(|e| format!("Export failed: {e}"))
}

/// Unpack a `.fuzz` file into a new folder under the default library directory and open it.
#[tauri::command]
async fn import_library(app: AppHandle, state: State<'_, AppState>, archive: String) -> Res<LibraryView> {
    let archive_path = PathBuf::from(archive);
    let base = PathBuf::from(state.config.lock().default_library_directory.clone());
    let root = tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<PathBuf> {
        let manifest = archive::read_manifest(&archive_path)?;
        let clean = match sanitize_name(&manifest.library_name) {
            n if n.is_empty() => "Imported library".to_string(),
            n => n,
        };
        let mut root = base.join(&clean);
        let mut i = 2;
        while root.exists() {
            root = base.join(format!("{clean} ({i})"));
            i += 1;
        }
        if let Err(e) = archive::import(&archive_path, &root, &emit_task(&app, "Importing")) {
            let _ = std::fs::remove_dir_all(&root); // never leave a half-imported library behind
            return Err(e);
        }
        Ok(root)
    })
    .await
    .map_err(err)?
    .map_err(|e| format!("Import failed: {e}"))?;
    let r = root.clone();
    let lib = tauri::async_runtime::spawn_blocking(move || Library::load_from_json(&r))
        .await
        .map_err(err)?
        .map_err(|e| format!("Import failed: {e}"))?;
    Ok(install(&state, root, lib))
}

#[tauri::command]
fn open_file(app: AppHandle, state: State<AppState>, file_id: String) -> Res<()> {
    let p = abs_path(&state, &file_id)?;
    app.opener().open_path(p.to_string_lossy(), None::<&str>).map_err(err)
}

#[tauri::command]
fn reveal_file(app: AppHandle, state: State<AppState>, file_id: String) -> Res<()> {
    let p = abs_path(&state, &file_id)?;
    app.opener().reveal_item_in_dir(p).map_err(err)
}

#[tauri::command]
fn reveal_root(app: AppHandle, state: State<AppState>) -> Res<()> {
    let g = state.lib.read();
    let l = g.as_ref().ok_or("No library open")?;
    app.opener().open_path(l.root.to_string_lossy(), None::<&str>).map_err(err)
}

#[tauri::command]
fn open_model_folder(app: AppHandle) -> Res<()> {
    let d = config::model_dir();
    std::fs::create_dir_all(&d).map_err(err)?;
    app.opener().open_path(d.to_string_lossy(), None::<&str>).map_err(err)
}

#[tauri::command]
async fn reload_model(state: State<'_, AppState>) -> Res<ModelStatus> {
    state.loading.store(true, Ordering::Relaxed);
    let emb = tauri::async_runtime::spawn_blocking(embed::default_embedder).await.map_err(err)?;
    *state.embedder.write() = emb;
    state.loading.store(false, Ordering::Relaxed);
    Ok(model_status(&state))
}

/// Re-send the library view (e.g. after the model finished loading and the mismatch state changed).
#[tauri::command]
fn library_view(state: State<AppState>) -> Res<LibraryView> {
    let g = state.lib.read();
    let l = g.as_ref().ok_or("No library open")?;
    Ok(view(&state, l))
}

/// Download the EmbeddingGemma ONNX model, then load it.
#[tauri::command]
async fn install_model(app: AppHandle, state: State<'_, AppState>) -> Res<ModelStatus> {
    let dir = config::model_dir();
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        model::install(&dir, &|done, total, cur| {
            let _ = app2.emit("task-progress", Progress { label: "Downloading model", done: done as usize, total: total as usize, current: cur.to_string() });
        })
    })
    .await
    .map_err(err)?
    .map_err(|e| e.to_string())?;
    reload_model(state).await
}

/// Raw PDF bytes for the in-app viewer (sent as binary, not JSON).
#[tauri::command]
async fn read_pdf(state: State<'_, AppState>, file_id: String) -> Res<tauri::ipc::Response> {
    let p = abs_path(&state, &file_id)?;
    if p.extension().and_then(|e| e.to_str()).map(|e| !e.eq_ignore_ascii_case("pdf")).unwrap_or(true) {
        return Err("Not a PDF".into());
    }
    let bytes = tauri::async_runtime::spawn_blocking(move || std::fs::read(p)).await.map_err(err)?.map_err(err)?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            config: Mutex::new(AppConfig::load()),
            lib: RwLock::new(None),
            embedder: RwLock::new(Arc::new(embed::HashEmbedder)),
            loading: AtomicBool::new(model::installed(&config::model_dir())),
        })
        .setup(|app| {
            // Loading a 300 MB model can take seconds: never block the window on it.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let state = handle.state::<AppState>();
                if state.loading.load(Ordering::Relaxed) {
                    let emb = embed::default_embedder();
                    *state.embedder.write() = emb;
                    state.loading.store(false, Ordering::Relaxed);
                }
                let _ = handle.emit("model-ready", model_status(&state));
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            set_default_directory,
            forget_recent,
            open_library,
            create_library,
            sync_library,
            search,
            set_tags,
            create_folder,
            move_files,
            import_files,
            export_library,
            import_library,
            open_file,
            reveal_file,
            reveal_root,
            open_model_folder,
            reload_model,
            library_view,
            install_model,
            read_pdf,
        ])
        .run(tauri::generate_context!())
        .expect("error while running fuzz");
}
