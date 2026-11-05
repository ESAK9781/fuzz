//! In-app installer for the EmbeddingGemma ONNX model (public onnx-community export).

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::time::{Duration, Instant};

const BASE: &str = "https://huggingface.co/onnx-community/embeddinggemma-300m-ONNX/resolve/main/";

/// (remote path, local file name, expected size in bytes). The size doubles as an integrity check.
pub const FILES: &[(&str, &str, u64)] = &[
    ("tokenizer.json", "tokenizer.json", 20_323_312),
    ("onnx/model_quantized.onnx", "model_quantized.onnx", 567_874),
    ("onnx/model_quantized.onnx_data", "model_quantized.onnx_data", 308_890_624),
];

pub fn total_bytes() -> u64 {
    FILES.iter().map(|f| f.2).sum()
}

pub fn installed(dir: &Path) -> bool {
    FILES
        .iter()
        .all(|(_, name, size)| std::fs::metadata(dir.join(name)).map(|m| m.len() == *size).unwrap_or(false))
}

/// Download all model files into `dir`. `progress(done_bytes, total_bytes, current_file)`.
pub fn install(dir: &Path, progress: &dyn Fn(u64, u64, &str)) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;
    let total = total_bytes();
    let mut done = 0u64;
    let mut last = Instant::now() - Duration::from_secs(1);
    for (remote, local, size) in FILES {
        let target = dir.join(local);
        if std::fs::metadata(&target).map(|m| m.len() == *size).unwrap_or(false) {
            done += size;
            continue;
        }
        let part = dir.join(format!("{local}.part"));
        let resp = ureq::get(&format!("{BASE}{remote}"))
            .call()
            .map_err(|e| anyhow::anyhow!("Could not download {local}: {e}. Check your internet connection."))?;
        let mut reader = resp.into_body().into_reader();
        let mut out = File::create(&part)?;
        let mut buf = vec![0u8; 256 * 1024];
        let mut got = 0u64;
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n])?;
            got += n as u64;
            if last.elapsed() > Duration::from_millis(120) {
                progress(done + got, total, local);
                last = Instant::now();
            }
        }
        out.flush()?;
        drop(out);
        if got != *size {
            let _ = std::fs::remove_file(&part);
            anyhow::bail!("Download of {local} was incomplete ({got} of {size} bytes). Please try again.");
        }
        std::fs::rename(&part, &target)?;
        done += size;
    }
    progress(total, total, "");
    Ok(())
}
