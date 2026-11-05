//! Single-file export/import of a library (`.fuzz`).
//!
//! The container is a standard ZIP (so it is portable across Windows/macOS/Linux and can be
//! inspected with any zip tool) holding `fuzz-manifest.json`, `.semantic_index.json` and every
//! (non-hidden) file of the library using `/`-separated relative paths. Media is stored
//! uncompressed (it is already compressed); only JSON is deflated.

use crate::library::INDEX_FILE;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

pub const MANIFEST: &str = "fuzz-manifest.json";
pub const FORMAT: &str = "fuzz-library-archive";
pub const FORMAT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
pub struct Manifest {
    pub format: String,
    pub format_version: u32,
    pub app_version: String,
    pub library_name: String,
    pub file_count: usize,
}

fn collect(root: &Path) -> Vec<(PathBuf, String)> {
    let walker = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !e.file_name().to_string_lossy().starts_with('.'));
    walker
        .flatten()
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            let rel = e.path().strip_prefix(root).ok()?.to_string_lossy().replace('\\', "/");
            Some((e.path().to_path_buf(), rel))
        })
        .collect()
}

pub fn export(root: &Path, name: &str, dest: &Path, progress: &dyn Fn(usize, usize, &str)) -> anyhow::Result<usize> {
    let files = collect(root);
    let total = files.len() + 2;
    let tmp = dest.with_extension("fuzz.part");
    let mut zip = ZipWriter::new(BufWriter::new(File::create(&tmp)?));
    let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored).large_file(true);
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    let manifest = Manifest {
        format: FORMAT.into(),
        format_version: FORMAT_VERSION,
        app_version: env!("CARGO_PKG_VERSION").into(),
        library_name: name.into(),
        file_count: files.len(),
    };
    zip.start_file(MANIFEST, deflated)?;
    zip.write_all(&serde_json::to_vec_pretty(&manifest)?)?;
    progress(1, total, MANIFEST);

    zip.start_file(INDEX_FILE, deflated.large_file(true))?;
    std::io::copy(&mut File::open(root.join(INDEX_FILE))?, &mut zip)?;
    progress(2, total, INDEX_FILE);

    for (i, (abs, rel)) in files.iter().enumerate() {
        progress(i + 3, total, rel);
        zip.start_file(rel, stored)?;
        std::io::copy(&mut BufReader::new(File::open(abs)?), &mut zip)?;
    }
    zip.finish()?.flush()?;
    std::fs::rename(&tmp, dest)?;
    Ok(files.len())
}

/// Read just the manifest (to validate and name the library) without extracting.
pub fn read_manifest(archive: &Path) -> anyhow::Result<Manifest> {
    let mut z = ZipArchive::new(BufReader::new(File::open(archive)?))?;
    let m: Manifest = serde_json::from_reader(z.by_name(MANIFEST).map_err(|_| anyhow::anyhow!("Not a fuzz library file"))?)?;
    if m.format != FORMAT {
        anyhow::bail!("Not a fuzz library file");
    }
    if m.format_version > FORMAT_VERSION {
        anyhow::bail!("This library was exported by a newer version of fuzz. Please update the app.");
    }
    Ok(m)
}

pub fn import(archive: &Path, dest: &Path, progress: &dyn Fn(usize, usize, &str)) -> anyhow::Result<()> {
    let mut z = ZipArchive::new(BufReader::new(File::open(archive)?))?;
    std::fs::create_dir_all(dest)?;
    let total = z.len();
    for i in 0..total {
        let mut f = z.by_index(i)?;
        // enclosed_name rejects absolute paths and `..` traversal (zip-slip).
        let Some(rel) = f.enclosed_name() else { continue };
        if rel.as_os_str() == MANIFEST || f.is_dir() {
            continue;
        }
        progress(i + 1, total, &rel.to_string_lossy());
        let out = dest.join(&rel);
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p)?;
        }
        std::io::copy(&mut f, &mut BufWriter::new(File::create(&out)?))?;
    }
    if !dest.join(INDEX_FILE).exists() {
        anyhow::bail!("Archive has no library index");
    }
    Ok(())
}
