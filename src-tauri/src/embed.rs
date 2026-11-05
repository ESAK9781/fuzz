use std::sync::Arc;
use unicode_normalization::char::is_combining_mark;
use unicode_normalization::UnicodeNormalization;

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
/// Unicode-aware: accents and compatibility forms are folded ("café" ≈ "cafe", "ﬁ" ≈ "fi"),
/// and CJK text (no spaces) is indexed as character unigrams + bigrams.
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

fn is_cjk(c: char) -> bool {
    matches!(c as u32, 0x3040..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF | 0x20000..=0x2A6DF)
}

/// Lowercase, compatibility-normalise and strip combining marks.
pub fn fold(s: &str) -> String {
    s.nfkd().filter(|c| !is_combining_mark(*c)).collect::<String>().to_lowercase()
}

/// (word tokens, CJK runs)
fn tokenize(text: &str) -> (Vec<String>, Vec<Vec<char>>) {
    let folded = fold(text);
    let mut words = vec![];
    let mut runs: Vec<Vec<char>> = vec![];
    let mut cur = String::new();
    let mut run: Vec<char> = vec![];
    for c in folded.chars() {
        if is_cjk(c) {
            if !cur.is_empty() {
                words.push(std::mem::take(&mut cur));
            }
            run.push(c);
            continue;
        }
        if !run.is_empty() {
            runs.push(std::mem::take(&mut run));
        }
        if c.is_alphanumeric() {
            cur.push(c);
        } else if !cur.is_empty() {
            words.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    if !run.is_empty() {
        runs.push(run);
    }
    (words.into_iter().filter(|w| !STOP.contains(&w.as_str())).collect(), runs)
}

impl Embedder for HashEmbedder {
    fn name(&self) -> &str {
        "fuzz-hash-256d"
    }

    fn embed_docs(&self, texts: &[String]) -> Vec<Vec<f32>> {
        texts.iter().map(|t| self.embed_query(t)).collect()
    }

    fn embed_query(&self, text: &str) -> Vec<f32> {
        let (words, runs) = tokenize(text);
        let words: Vec<&str> = words.iter().map(|w| stem(w)).collect();
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
        for run in &runs {
            for c in run {
                add(&c.to_string(), 0.5);
            }
            for pair in run.windows(2) {
                add(&pair.iter().collect::<String>(), 1.0);
            }
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
