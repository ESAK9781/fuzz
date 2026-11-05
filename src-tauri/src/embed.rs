use std::sync::Arc;

pub const DIM: usize = 256;

pub trait Embedder: Send + Sync {
    /// Identifier stored in the index; a mismatch triggers re-embedding.
    fn name(&self) -> &str;
    fn accelerator(&self) -> String {
        "CPU (multi-threaded)".into()
    }
    fn embed_docs(&self, texts: &[String]) -> Vec<Vec<f32>>;
    fn embed_query(&self, text: &str) -> Vec<f32>;
}

pub fn normalize(v: &mut [f32]) {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > 0.0 {
        v.iter_mut().for_each(|x| *x /= n);
    }
}

/// Offline fallback: signed feature hashing of word uni/bi-grams and char trigrams.
/// Captures lexical overlap (not true semantics); used when the ONNX model is absent.
pub struct HashEmbedder;

fn fnv(s: &str, seed: u64) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325 ^ seed;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn stem(w: &str) -> &str {
    for suf in ["ing", "ed", "es", "s", "ly"] {
        if w.len() > suf.len() + 3 && w.ends_with(suf) {
            return &w[..w.len() - suf.len()];
        }
    }
    w
}

const STOP: &[&str] = &[
    "the", "a", "an", "and", "or", "of", "to", "in", "is", "are", "was", "were", "for", "on", "with",
    "as", "by", "at", "it", "this", "that", "be", "from", "we", "our", "can",
];

impl Embedder for HashEmbedder {
    fn name(&self) -> &str {
        "fuzz-hash-256d"
    }

    fn embed_docs(&self, texts: &[String]) -> Vec<Vec<f32>> {
        texts.iter().map(|t| self.embed_query(t)).collect()
    }

    fn embed_query(&self, text: &str) -> Vec<f32> {
        let lower = text.to_lowercase();
        let words: Vec<&str> = lower
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty() && !STOP.contains(w))
            .map(stem)
            .collect();
        let mut v = vec![0f32; DIM];
        let mut add = |feat: &str, w: f32| {
            let h = fnv(feat, 0);
            let sign = if fnv(feat, 0x9e3779b97f4a7c15) & 1 == 0 { 1.0 } else { -1.0 };
            v[(h % DIM as u64) as usize] += sign * w;
        };
        for w in &words {
            add(w, 1.0);
            let padded: Vec<char> = format!("^{w}$").chars().collect();
            for tri in padded.windows(3) {
                add(&tri.iter().collect::<String>(), 0.35);
            }
        }
        for pair in words.windows(2) {
            add(&format!("{} {}", pair[0], pair[1]), 0.6);
        }
        // sublinear damping so long chunks don't dominate
        v.iter_mut().for_each(|x| *x = x.signum() * x.abs().sqrt());
        normalize(&mut v);
        v
    }
}

pub fn default_embedder() -> Arc<dyn Embedder> {
    #[cfg(feature = "onnx")]
    {
        if let Some(e) = crate::onnx::OnnxEmbedder::try_load(&crate::config::model_dir()) {
            return Arc::new(e);
        }
    }
    Arc::new(HashEmbedder)
}
