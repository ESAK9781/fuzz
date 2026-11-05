use crate::embed::{Embedder, HashEmbedder};
use crate::extract::{parse_cues_for_test, split_sentences};
use crate::indexer;
use crate::library::*;
use crate::search::{search, SearchParams};

fn tmp(name: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("fuzz-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn params(q: &str, scope: &str) -> SearchParams {
    SearchParams {
        query: q.into(),
        types: ["pdf", "text", "audio", "video", "image"].map(String::from).to_vec(),
        scope: scope.into(),
        tags: vec![],
        folder: None,
        limit: 10,
    }
}

#[test]
fn sentences_split_and_merge_short() {
    let s = split_sentences("Transformers use attention. Ok. They scale well with data and compute! Done?");
    assert!(s.len() >= 2, "{s:?}");
    assert!(s.iter().all(|x| x.chars().count() >= 3));
}

#[test]
fn srt_cues_parse_with_timestamps() {
    let cues = parse_cues_for_test("1\n00:00:01,500 --> 00:00:04,000\nHello <i>world</i>\n\n2\n00:01:00,000 --> 00:01:02,000\nSecond cue\n");
    assert_eq!(cues, vec![(1.5, 4.0, "Hello world".to_string()), (60.0, 62.0, "Second cue".to_string())]);
}

#[test]
fn hash_embedder_prefers_related_text() {
    let e = HashEmbedder;
    let docs = e.embed_docs(&["Neural networks learn representations with gradient descent".into(), "Sourdough bread needs a long fermentation".into()]);
    let q = e.embed_query("how do neural networks learn");
    let dot = |a: &[f32], b: &[f32]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>();
    assert!(dot(&q, &docs[0]) > dot(&q, &docs[1]));
}

#[test]
fn sync_search_tags_folders_and_rename() {
    let root = tmp("sync");
    std::fs::create_dir_all(root.join("science")).unwrap();
    std::fs::write(root.join("science/attention.txt"), "Transformers rely on self-attention mechanisms to compute representations. Recurrent networks process tokens sequentially instead.").unwrap();
    std::fs::write(root.join("bread.md"), "Sourdough bread requires a long fermentation with wild yeast. Bake it in a hot dutch oven.").unwrap();
    std::fs::write(root.join("talk.mp3"), b"not really audio").unwrap();
    std::fs::write(root.join("talk.srt"), "1\n00:00:05,000 --> 00:00:09,000\nToday we discuss gradient descent optimisation in detail.\n").unwrap();

    let emb = HashEmbedder;
    let lib = Library::empty("t");
    let (lib, stats) = indexer::sync(&root, &lib, &emb, &|_, _, _| {});
    assert_eq!(stats.added, 3);
    assert!(lib.folders.contains(&"science".to_string()));
    assert!(!lib.files.values().any(|f| f.path.ends_with(".srt")), "sidecars are not files");

    let mut l = Loaded::new(root.clone(), lib);
    let hits = search(&l, &emb.embed_query("self-attention transformers"), &params("x", "child"));
    assert_eq!(hits[0].path, "science/attention.txt");
    assert!(hits[0].parent_text.is_some());

    let hits = search(&l, &emb.embed_query("gradient descent"), &params("x", "child"));
    assert_eq!(hits[0].path, "talk.mp3");
    assert_eq!(hits[0].start_sec, Some(5.0));

    // tag + folder filters
    let id = l.lib.files.iter().find(|(_, f)| f.path == "bread.md").unwrap().0.clone();
    l.lib.files.get_mut(&id).unwrap().tags.push("bookmark".into());
    let mut p = params("bread", "file");
    p.tags = vec!["bookmark".into()];
    let hits = search(&l, &emb.embed_query("bread"), &p);
    assert_eq!(hits.len(), 1);
    let mut p = params("bread", "file");
    p.folder = Some("science".into());
    assert!(search(&l, &emb.embed_query("bread"), &p).iter().all(|h| h.path.starts_with("science/")));

    // move on disk outside the app: id and tags survive, nothing re-embedded
    std::fs::create_dir_all(root.join("food")).unwrap();
    std::fs::rename(root.join("bread.md"), root.join("food/bread.md")).unwrap();
    let (lib2, stats2) = indexer::sync(&root, &l.lib, &emb, &|_, _, _| {});
    assert_eq!((stats2.added, stats2.updated), (0, 0));
    let f = lib2.files.get(&id).expect("same id after rename");
    assert_eq!(f.path, "food/bread.md");
    assert!(f.tags.contains(&"bookmark".to_string()));

    // json round trip
    lib2.save_to_json(&root).unwrap();
    let back = Library::load_from_json(&root).unwrap();
    assert_eq!(back.chunks.len(), lib2.chunks.len());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn clean_rel_rejects_traversal() {
    assert_eq!(clean_rel(r"a/b\c"), Some("a/b/c".into()));
    assert_eq!(clean_rel("../x"), None);
    assert_eq!(clean_rel(""), Some("".into()));
}

#[test]
fn archive_round_trip_is_portable() {
    let root = tmp("arch-src");
    std::fs::create_dir_all(root.join("a/b")).unwrap();
    std::fs::write(root.join("a/b/note.txt"), "Gradient descent minimises a loss function step by step.").unwrap();
    std::fs::write(root.join(".hidden"), "skip me").unwrap();
    let emb = HashEmbedder;
    let (lib, _) = indexer::sync(&root, &Library::empty("My Lib"), &emb, &|_, _, _| {});
    lib.save_to_json(&root).unwrap();

    let out = tmp("arch-out");
    let file = out.join("x.fuzz");
    let n = crate::archive::export(&root, "My Lib", &file, &|_, _, _| {}).unwrap();
    assert_eq!(n, 1, "only the note; the index is written separately and hidden files are skipped");
    let m = crate::archive::read_manifest(&file).unwrap();
    assert_eq!(m.library_name, "My Lib");

    let dest = out.join("imported");
    crate::archive::import(&file, &dest, &|_, _, _| {}).unwrap();
    assert!(dest.join("a/b/note.txt").exists());
    assert!(!dest.join(".hidden").exists());
    let back = Library::load_from_json(&dest).unwrap();
    assert_eq!(back.chunks.len(), lib.chunks.len());
    assert_eq!(back.files.len(), 1);
    // an unchanged imported library syncs with zero re-embedding
    let (_, st) = indexer::sync(&dest, &back, &emb, &|_, _, _| {});
    assert_eq!((st.added, st.updated, st.removed), (0, 0, 0));
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&out);
}
