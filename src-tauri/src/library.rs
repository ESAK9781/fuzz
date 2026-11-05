use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

pub const INDEX_FILE: &str = ".semantic_index.json";
pub const LIBRARY_VERSION: &str = "1.0.0";

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FileEntry {
    /// Path relative to the library root, always using `/` separators.
    pub path: String,
    pub media_type: String,
    pub file_hash: String,
    pub last_modified: i64,
    #[serde(default)]
    pub size: u64,
    pub item_chunk_id: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Whether a transcript / caption / text could be extracted.
    #[serde(default = "yes")]
    pub has_text: bool,
}

fn yes() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Chunk {
    pub id: String,
    pub file_id: String,
    /// "file" | "parent" | "child"
    pub granularity: String,
    pub text: String,
    pub parent_id: Option<String>,
    pub page: Option<u32>,
    pub start_sec: Option<f64>,
    pub end_sec: Option<f64>,
    pub vector: Vec<f32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Library {
    pub library_version: String,
    pub name: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub embedding_model: String,
    #[serde(default)]
    pub folders: Vec<String>,
    pub files: BTreeMap<String, FileEntry>,
    pub chunks: Vec<Chunk>,
}

impl Library {
    pub fn empty(name: &str) -> Self {
        let t = now();
        Library {
            library_version: LIBRARY_VERSION.into(),
            name: name.into(),
            created_at: t,
            updated_at: t,
            embedding_model: String::new(),
            folders: vec![],
            files: BTreeMap::new(),
            chunks: vec![],
        }
    }

    pub fn load_from_json(root: &Path) -> anyhow::Result<Self> {
        let data = std::fs::read(root.join(INDEX_FILE))?;
        let lib: Library = serde_json::from_slice(&data)?;
        let major = |v: &str| v.split('.').next().and_then(|m| m.parse::<u32>().ok()).unwrap_or(1);
        if major(&lib.library_version) > major(LIBRARY_VERSION) {
            anyhow::bail!("this library was created by a newer version of fuzz ({}). Please update the app.", lib.library_version);
        }
        Ok(lib)
    }

    /// Atomic save: write to a temp file and rename over the index.
    pub fn save_to_json(&self, root: &Path) -> anyhow::Result<()> {
        let tmp = root.join(".semantic_index.json.tmp");
        std::fs::write(&tmp, serde_json::to_vec(self)?)?;
        std::fs::rename(&tmp, root.join(INDEX_FILE))?;
        Ok(())
    }
}

/// A loaded library plus the contiguous vector buffer used for scoring.
pub struct Loaded {
    pub root: PathBuf,
    pub lib: Library,
    pub dim: usize,
    /// Row-major, `chunks.len() * dim`, rows are L2-normalised.
    pub flat: Vec<f32>,
    pub chunk_by_id: HashMap<String, usize>,
}

impl Loaded {
    pub fn new(root: PathBuf, lib: Library) -> Self {
        let mut l = Loaded {
            root,
            lib,
            dim: 0,
            flat: vec![],
            chunk_by_id: HashMap::new(),
        };
        l.rebuild();
        l
    }

    pub fn rebuild(&mut self) {
        self.dim = self.lib.chunks.iter().map(|c| c.vector.len()).max().unwrap_or(0);
        let dim = self.dim;
        let mut flat = vec![0f32; self.lib.chunks.len() * dim];
        let mut idx = HashMap::with_capacity(self.lib.chunks.len());
        for (i, c) in self.lib.chunks.iter().enumerate() {
            let row = &mut flat[i * dim..i * dim + c.vector.len().min(dim)];
            row.copy_from_slice(&c.vector[..row.len()]);
            let n = row.iter().map(|x| x * x).sum::<f32>().sqrt();
            if n > 0.0 {
                row.iter_mut().for_each(|x| *x /= n);
            }
            idx.insert(c.id.clone(), i);
        }
        self.flat = flat;
        self.chunk_by_id = idx;
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        self.lib.updated_at = now();
        self.lib.save_to_json(&self.root)
    }
}

pub fn media_type_for(ext: &str) -> Option<&'static str> {
    match ext.to_ascii_lowercase().as_str() {
        "pdf" => Some("pdf"),
        "txt" | "md" | "markdown" | "rst" | "log" | "org" | "tex" => Some("text"),
        "mp3" | "wav" | "flac" | "m4a" | "aac" | "ogg" | "opus" | "wma" => Some("audio"),
        "mp4" | "mkv" | "mov" | "avi" | "webm" | "m4v" | "wmv" => Some("video"),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "tif" | "tiff" | "svg" | "heic" => {
            Some("image")
        }
        _ => None,
    }
}

/// Sidecar files (transcripts/captions) are consumed by their media file, not indexed alone.
pub fn is_sidecar(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.ends_with(".srt") || n.ends_with(".vtt") || n.ends_with(".transcript.txt") || n.ends_with(".caption.txt")
}

/// Strip characters that are illegal in folder/file names on common platforms.
pub fn sanitize_name(s: &str) -> String {
    s.trim()
        .chars()
        .map(|c| if "<>:\"/\\|?*".contains(c) || c.is_control() { '_' } else { c })
        .collect::<String>()
        .trim_matches('.')
        .to_string()
}

/// Normalise a user-supplied relative folder path and reject traversal.
pub fn clean_rel(s: &str) -> Option<String> {
    let parts: Vec<String> = s
        .split(['/', '\\'])
        .filter(|p| !p.is_empty())
        .map(sanitize_name)
        .collect();
    if parts.iter().any(|p| p.is_empty() || p == "..") {
        return None;
    }
    Some(parts.join("/"))
}
