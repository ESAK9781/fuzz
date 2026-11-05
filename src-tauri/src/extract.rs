//! Turn a file into text units: (parent passages, child sentences) with optional page / timestamps.

use std::path::Path;

pub struct Parent {
    pub text: String,
    pub page: Option<u32>,
    pub start: Option<f64>,
    pub end: Option<f64>,
    pub children: Vec<Child>,
}

pub struct Child {
    pub text: String,
    pub start: Option<f64>,
    pub end: Option<f64>,
}

pub struct Extracted {
    /// Text pooled for the file-level chunk.
    pub summary: String,
    pub parents: Vec<Parent>,
    pub has_text: bool,
}

const PARENT_MAX: usize = 700;
const SUMMARY_MAX: usize = 1800;

pub fn split_sentences(text: &str) -> Vec<String> {
    let mut out = vec![];
    let mut cur = String::new();
    let chars: Vec<char> = text.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        cur.push(c);
        let end_punct = matches!(c, '.' | '!' | '?' | '。' | '！' | '？');
        let next = chars.get(i + 1);
        let boundary = (end_punct && next.map_or(true, |n| n.is_whitespace()))
            || (c == '\n' && next == Some(&'\n'));
        if boundary {
            push_sentence(&mut out, &mut cur);
        }
    }
    push_sentence(&mut out, &mut cur);
    // merge fragments that are too short into the previous sentence
    let mut merged: Vec<String> = vec![];
    for s in out {
        match merged.last_mut() {
            Some(prev) if s.chars().count() < 25 => {
                prev.push(' ');
                prev.push_str(&s);
            }
            _ => merged.push(s),
        }
    }
    merged
}

fn push_sentence(out: &mut Vec<String>, cur: &mut String) {
    let s = cur.split_whitespace().collect::<Vec<_>>().join(" ");
    cur.clear();
    if s.chars().filter(|c| c.is_alphanumeric()).count() >= 3 {
        out.push(s);
    }
}

/// Group sentences into passages of roughly PARENT_MAX chars.
fn group(sentences: Vec<String>, page: Option<u32>) -> Vec<Parent> {
    let mut parents = vec![];
    let mut cur: Vec<String> = vec![];
    let mut len = 0;
    let flush = |cur: &mut Vec<String>, parents: &mut Vec<Parent>| {
        if cur.is_empty() {
            return;
        }
        let kids = std::mem::take(cur);
        parents.push(Parent {
            text: kids.join(" "),
            page,
            start: None,
            end: None,
            children: kids.into_iter().map(|t| Child { text: t, start: None, end: None }).collect(),
        });
    };
    for s in sentences {
        if len + s.len() > PARENT_MAX && !cur.is_empty() {
            flush(&mut cur, &mut parents);
            len = 0;
        }
        len += s.len();
        cur.push(s);
    }
    flush(&mut cur, &mut parents);
    parents
}

fn summarize(parents: &[Parent], fallback: &str) -> String {
    let mut s = String::new();
    for p in parents {
        if s.len() >= SUMMARY_MAX {
            break;
        }
        s.push_str(&p.text);
        s.push(' ');
    }
    if s.trim().is_empty() {
        return fallback.to_string();
    }
    s.chars().take(SUMMARY_MAX).collect()
}

fn title_from(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().replace(['_', '-', '.'], " "))
        .unwrap_or_default()
}

pub fn extract(path: &Path, media_type: &str) -> Extracted {
    let title = title_from(path);
    let (parents, has_text) = match media_type {
        "text" => {
            let raw = std::fs::read(path).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
            let ps = group(split_sentences(&raw), None);
            let ok = !ps.is_empty();
            (ps, ok)
        }
        "pdf" => {
            let ps = pdf_parents(path);
            let ok = !ps.is_empty();
            (ps, ok)
        }
        "audio" | "video" => {
            let ps = transcript_parents(path);
            let ok = !ps.is_empty();
            (ps, ok)
        }
        _ => {
            // images: optional caption sidecar
            let cap = sidecar_path(path, "caption.txt")
                .and_then(|p| std::fs::read_to_string(p).ok())
                .unwrap_or_default();
            let ps = group(split_sentences(&cap), None);
            let ok = !ps.is_empty();
            (ps, ok)
        }
    };
    let summary = format!("{title}. {}", summarize(&parents, ""));
    Extracted { summary, parents, has_text }
}

fn pdf_parents(path: &Path) -> Vec<Parent> {
    let path = path.to_path_buf();
    let pages = std::panic::catch_unwind(move || pdf_extract::extract_text_by_pages(&path))
        .ok()
        .and_then(|r| r.ok())
        .unwrap_or_default();
    let mut out = vec![];
    for (i, text) in pages.iter().enumerate() {
        out.extend(group(split_sentences(text), Some(i as u32 + 1)));
    }
    out
}

fn sidecar_path(path: &Path, ext: &str) -> Option<std::path::PathBuf> {
    let stem = path.file_stem()?.to_string_lossy().into_owned();
    let p = path.with_file_name(format!("{stem}.{ext}"));
    p.exists().then_some(p)
}

fn transcript_parents(path: &Path) -> Vec<Parent> {
    for ext in ["srt", "vtt"] {
        if let Some(p) = sidecar_path(path, ext) {
            if let Ok(raw) = std::fs::read_to_string(p) {
                let cues = parse_cues(&raw);
                if !cues.is_empty() {
                    return group_cues(cues);
                }
            }
        }
    }
    if let Some(p) = sidecar_path(path, "transcript.txt") {
        if let Ok(raw) = std::fs::read_to_string(p) {
            return group(split_sentences(&raw), None);
        }
    }
    vec![]
}

struct Cue {
    start: f64,
    end: f64,
    text: String,
}

fn parse_ts(s: &str) -> Option<f64> {
    let s = s.trim().replace(',', ".");
    let parts: Vec<&str> = s.split(':').collect();
    let nums: Vec<f64> = parts.iter().filter_map(|p| p.parse().ok()).collect();
    if nums.len() != parts.len() {
        return None;
    }
    Some(nums.iter().fold(0.0, |acc, n| acc * 60.0 + n))
}

fn parse_cues(raw: &str) -> Vec<Cue> {
    let mut cues = vec![];
    let lines: Vec<&str> = raw.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        if let Some((a, b)) = lines[i].split_once("-->") {
            let b = b.split_whitespace().next().unwrap_or("");
            if let (Some(start), Some(end)) = (parse_ts(a), parse_ts(b)) {
                let mut text = vec![];
                i += 1;
                while i < lines.len() && !lines[i].trim().is_empty() {
                    text.push(lines[i].trim());
                    i += 1;
                }
                let t = strip_tags(&text.join(" "));
                if !t.is_empty() {
                    cues.push(Cue { start, end, text: t });
                }
                continue;
            }
        }
        i += 1;
    }
    cues
}

fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn group_cues(cues: Vec<Cue>) -> Vec<Parent> {
    let mut parents: Vec<Parent> = vec![];
    let mut cur: Vec<Cue> = vec![];
    let mut len = 0;
    let flush = |cur: &mut Vec<Cue>, parents: &mut Vec<Parent>| {
        if cur.is_empty() {
            return;
        }
        let kids = std::mem::take(cur);
        parents.push(Parent {
            text: kids.iter().map(|c| c.text.as_str()).collect::<Vec<_>>().join(" "),
            page: None,
            start: Some(kids[0].start),
            end: Some(kids[kids.len() - 1].end),
            children: kids
                .into_iter()
                .map(|c| Child { text: c.text, start: Some(c.start), end: Some(c.end) })
                .collect(),
        });
    };
    for c in cues {
        if len + c.text.len() > PARENT_MAX && !cur.is_empty() {
            flush(&mut cur, &mut parents);
            len = 0;
        }
        len += c.text.len();
        cur.push(c);
    }
    flush(&mut cur, &mut parents);
    parents
}

#[cfg(test)]
pub fn parse_cues_for_test(raw: &str) -> Vec<(f64, f64, String)> {
    parse_cues(raw).into_iter().map(|c| (c.start, c.end, c.text)).collect()
}
