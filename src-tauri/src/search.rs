use crate::library::Loaded;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct SearchParams {
    pub query: String,
    pub types: Vec<String>,
    /// "child" | "parent" | "file"
    pub scope: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub folder: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    40
}

#[derive(Serialize)]
pub struct Hit {
    pub chunk_id: String,
    pub file_id: String,
    pub path: String,
    pub media_type: String,
    pub granularity: String,
    pub text: String,
    pub parent_text: Option<String>,
    pub page: Option<u32>,
    pub start_sec: Option<f64>,
    pub end_sec: Option<f64>,
    pub score: f32,
    pub tags: Vec<String>,
}

#[inline]
fn dot(a: &[f32], b: &[f32]) -> f32 {
    // Four accumulators let the compiler auto-vectorise into SIMD lanes.
    let mut acc = [0f32; 4];
    let n = a.len().min(b.len());
    let (ca, cb) = (a[..n].chunks_exact(4), b[..n].chunks_exact(4));
    let (ra, rb) = (ca.remainder(), cb.remainder());
    for (x, y) in ca.zip(cb) {
        for k in 0..4 {
            acc[k] += x[k] * y[k];
        }
    }
    let tail: f32 = ra.iter().zip(rb).map(|(x, y)| x * y).sum();
    acc.iter().sum::<f32>() + tail
}

pub fn in_folder(path: &str, folder: &str) -> bool {
    folder.is_empty() || path.starts_with(&format!("{folder}/"))
}

pub fn search(l: &Loaded, q: &[f32], p: &SearchParams) -> Vec<Hit> {
    if l.dim == 0 || q.is_empty() {
        return vec![];
    }
    let mut query = q.to_vec();
    query.resize(l.dim, 0.0);
    crate::embed::normalize(&mut query);

    // Filter first, then score only the survivors.
    let candidates: Vec<usize> = l
        .lib
        .chunks
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            c.granularity == p.scope
                && l.lib.files.get(&c.file_id).is_some_and(|f| {
                    p.types.contains(&f.media_type)
                        && p.tags.iter().all(|t| f.tags.contains(t))
                        && p.folder.as_deref().map_or(true, |d| in_folder(&f.path, d))
                })
        })
        .map(|(i, _)| i)
        .collect();

    let mut scored: Vec<(usize, f32)> = candidates
        .par_iter()
        .map(|&i| (i, dot(&l.flat[i * l.dim..(i + 1) * l.dim], &query)))
        .collect();
    scored.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));

    // Keep the best chunk per (file, parent) so one passage can't flood the list.
    let mut seen = std::collections::HashSet::new();
    let mut hits = vec![];
    for (i, score) in scored {
        let c = &l.lib.chunks[i];
        let key = (c.file_id.clone(), c.parent_id.clone().unwrap_or_else(|| c.id.clone()));
        if !seen.insert(key) {
            continue;
        }
        let f = &l.lib.files[&c.file_id];
        let parent_text = c
            .parent_id
            .as_ref()
            .and_then(|pid| l.chunk_by_id.get(pid))
            .map(|&pi| l.lib.chunks[pi].text.clone());
        hits.push(Hit {
            chunk_id: c.id.clone(),
            file_id: c.file_id.clone(),
            path: f.path.clone(),
            media_type: f.media_type.clone(),
            granularity: c.granularity.clone(),
            text: c.text.clone(),
            parent_text,
            page: c.page,
            start_sec: c.start_sec,
            end_sec: c.end_sec,
            score,
            tags: f.tags.clone(),
        });
        if hits.len() >= p.limit {
            break;
        }
    }
    hits
}
