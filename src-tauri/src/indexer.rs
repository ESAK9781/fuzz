use crate::embed::Embedder;
use crate::extract;
use crate::library::*;
use rayon::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

struct Scanned {
    rel: String,
    media_type: &'static str,
    mtime: i64,
    size: u64,
}

fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

/// Walk the library root. Returns supported files and every sub-directory (relative).
fn scan(root: &Path) -> (Vec<Scanned>, Vec<String>) {
    let mut files = vec![];
    let mut dirs = vec![];
    let walker = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !is_hidden(&e.file_name().to_string_lossy()));
    for e in walker.flatten() {
        let rel = match e.path().strip_prefix(root) {
            Ok(r) if !r.as_os_str().is_empty() => r.to_string_lossy().replace('\\', "/"),
            _ => continue,
        };
        if e.file_type().is_dir() {
            dirs.push(rel);
            continue;
        }
        let name = e.file_name().to_string_lossy().into_owned();
        if is_sidecar(&name) {
            continue;
        }
        let Some(mt) = e.path().extension().and_then(|x| x.to_str()).and_then(media_type_for) else {
            continue;
        };
        let Ok(meta) = e.metadata() else { continue };
        let mtime = meta
            .modified()
            .ok()
            .and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        files.push(Scanned { rel, media_type: mt, mtime, size: meta.len() });
    }
    (files, dirs)
}

/// Cheap content fingerprint: SHA-256 of size + first 4 MiB (fast even for large video).
fn hash_file(path: &Path, size: u64) -> String {
    let mut h = Sha256::new();
    h.update(size.to_le_bytes());
    if let Ok(f) = std::fs::File::open(path) {
        let mut buf = Vec::new();
        let _ = f.take(4 << 20).read_to_end(&mut buf);
        h.update(&buf);
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect::<String>()
}

fn round5(v: Vec<f32>) -> Vec<f32> {
    v.into_iter().map(|x| (x * 1e5).round() / 1e5).collect()
}

fn max_chunk_num(chunks: &[Chunk]) -> u64 {
    chunks
        .iter()
        .filter_map(|c| c.id.strip_prefix("c_").and_then(|n| n.parse::<u64>().ok()))
        .max()
        .unwrap_or(0)
}

fn build_chunks(
    file_id: &str,
    path: &Path,
    media_type: &str,
    emb: &dyn Embedder,
    next: &AtomicU64,
) -> (Vec<Chunk>, String, bool) {
    let ex = extract::extract(path, media_type);
    let id = || format!("c_{}", next.fetch_add(1, Ordering::Relaxed) + 1);
    let mut metas: Vec<Chunk> = vec![];
    let file_chunk_id = id();
    metas.push(Chunk {
        id: file_chunk_id.clone(),
        file_id: file_id.into(),
        granularity: "file".into(),
        text: ex.summary,
        parent_id: None,
        page: None,
        start_sec: None,
        end_sec: None,
        vector: vec![],
    });
    for p in ex.parents {
        let pid = id();
        metas.push(Chunk {
            id: pid.clone(),
            file_id: file_id.into(),
            granularity: "parent".into(),
            text: p.text,
            parent_id: None,
            page: p.page,
            start_sec: p.start,
            end_sec: p.end,
            vector: vec![],
        });
        for c in p.children {
            metas.push(Chunk {
                id: id(),
                file_id: file_id.into(),
                granularity: "child".into(),
                text: c.text,
                parent_id: Some(pid.clone()),
                page: p.page,
                start_sec: c.start,
                end_sec: c.end,
                vector: vec![],
            });
        }
    }
    let texts: Vec<String> = metas.iter().map(|c| c.text.clone()).collect();
    let mut vecs = Vec::with_capacity(texts.len());
    for batch in texts.chunks(32) {
        vecs.extend(emb.embed_docs(batch));
    }
    for (c, v) in metas.iter_mut().zip(vecs) {
        c.vector = round5(v);
    }
    (metas, file_chunk_id, ex.has_text)
}

pub struct SyncStats {
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
}

/// Reconcile `lib` with the files on disk. Reuses unchanged work and tracks renames by hash.
pub fn sync(
    root: &Path,
    lib: &Library,
    emb: &dyn Embedder,
    progress: &(dyn Fn(usize, usize, &str) + Sync),
) -> (Library, SyncStats) {
    let (scanned, dirs) = scan(root);
    let model_changed = lib.embedding_model != emb.name();

    let mut old_chunks: HashMap<&str, Vec<&Chunk>> = HashMap::new();
    for c in &lib.chunks {
        old_chunks.entry(c.file_id.as_str()).or_default().push(c);
    }
    let old_by_path: HashMap<&str, (&String, &FileEntry)> =
        lib.files.iter().map(|(id, f)| (f.path.as_str(), (id, f))).collect();
    let on_disk: HashSet<&str> = scanned.iter().map(|s| s.rel.as_str()).collect();
    // orphans: indexed files no longer at their old path (candidates for rename detection)
    let mut orphans: HashMap<String, (&String, &FileEntry)> = lib
        .files
        .iter()
        .filter(|(_, f)| !on_disk.contains(f.path.as_str()))
        .map(|(id, f)| (format!("{}:{}", f.file_hash, f.size), (id, f)))
        .collect();

    let mut out = Library {
        library_version: LIBRARY_VERSION.into(),
        name: lib.name.clone(),
        created_at: lib.created_at,
        updated_at: now(),
        embedding_model: emb.name().to_string(),
        folders: lib.folders.iter().cloned().filter(|f| root.join(f).is_dir()).collect(),
        files: BTreeMap::new(),
        chunks: vec![],
    };
    for d in dirs {
        if !out.folders.contains(&d) {
            out.folders.push(d);
        }
    }
    out.folders.sort();

    let next = AtomicU64::new(max_chunk_num(&lib.chunks));
    let mut stats = SyncStats { added: 0, updated: 0, removed: 0 };
    let total = scanned.len();

    struct Work {
        rel: String,
        media_type: &'static str,
        mtime: i64,
        size: u64,
        id: String,
        tags: Vec<String>,
        hash: String,
        is_new: bool,
    }
    let mut work: Vec<Work> = vec![];
    let mut pending_ids: HashSet<String> = HashSet::new();
    let mut kept = 0usize;

    // Phase 1 (sequential, cheap): decide what can be reused.
    for s in &scanned {
        let abs = root.join(&s.rel);
        let existing = old_by_path.get(s.rel.as_str()).copied();
        let mut keep = |id: &String, f: FileEntry, out: &mut Library| {
            out.chunks.extend(old_chunks.get(id.as_str()).into_iter().flatten().map(|c| (*c).clone()));
            out.files.insert(id.clone(), f);
            kept += 1;
        };

        if let Some((id, f)) = existing {
            if !model_changed && f.last_modified == s.mtime && f.size == s.size {
                keep(id, f.clone(), &mut out);
                continue;
            }
        }
        let hash = hash_file(&abs, s.size);
        if let Some((id, f)) = existing {
            if !model_changed && f.file_hash == hash {
                let mut nf = f.clone();
                nf.last_modified = s.mtime;
                keep(id, nf, &mut out);
                continue;
            }
        }
        if existing.is_none() && !model_changed {
            if let Some((id, f)) = orphans.remove(&format!("{hash}:{}", s.size)) {
                let mut nf = f.clone();
                nf.path = s.rel.clone();
                nf.last_modified = s.mtime;
                keep(id, nf, &mut out);
                continue;
            }
        }
        let (mut id, tags, is_new) = match existing {
            Some((id, f)) => (id.clone(), f.tags.clone(), false),
            None => (format!("f_{}", &hash[..10]), vec![], true),
        };
        if is_new {
            while out.files.contains_key(&id) || lib.files.contains_key(&id) || pending_ids.contains(&id) {
                id.push('x');
            }
        }
        pending_ids.insert(id.clone());
        work.push(Work { rel: s.rel.clone(), media_type: s.media_type, mtime: s.mtime, size: s.size, id, tags, hash, is_new });
    }

    // Phase 2 (parallel): extract + embed. A dedicated pool leaves one core for the UI.
    let done = AtomicUsize::new(kept);
    progress(kept, total, "");
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2).saturating_sub(1).max(1);
    let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build();
    let run = |w: Work| {
        let (chunks, file_chunk, has_text) =
            build_chunks(&w.id, &root.join(&w.rel), w.media_type, emb, &next);
        let n = done.fetch_add(1, Ordering::Relaxed) + 1;
        progress(n, total, &w.rel);
        (w, chunks, file_chunk, has_text)
    };
    let results: Vec<_> = match &pool {
        Ok(p) => p.install(|| work.into_par_iter().map(run).collect()),
        Err(_) => work.into_iter().map(run).collect(),
    };

    for (w, chunks, file_chunk, has_text) in results {
        out.chunks.extend(chunks);
        out.files.insert(
            w.id,
            FileEntry {
                path: w.rel.clone(),
                media_type: w.media_type.into(),
                file_hash: w.hash,
                last_modified: w.mtime,
                size: w.size,
                item_chunk_id: Some(file_chunk),
                tags: w.tags,
                has_text,
            },
        );
        if w.is_new {
            stats.added += 1;
        } else {
            stats.updated += 1;
        }
    }
    stats.removed = lib.files.keys().filter(|id| !out.files.contains_key(*id)).count();
    progress(total, total, "");
    (out, stats)
}
