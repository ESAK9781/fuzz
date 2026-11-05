//! EmbeddingGemma (ONNX) embedder. Uses GPU execution providers when present
//! (CUDA / DirectML / CoreML) and silently falls back to CPU.
//!
//! Expected files in the model directory:
//!   model.onnx (or model_quantized.onnx / model_q4.onnx) and tokenizer.json

use crate::embed::{normalize, Embedder, DIM};
use ort::ep::{ExecutionProviderDispatch, CPU};
use ort::session::Session;
use ort::value::Tensor;
use parking_lot::Mutex;
use std::path::Path;
use tokenizers::{PaddingParams, Tokenizer, TruncationParams};

const MAX_TOKENS: usize = 512;
const NAME: &str = "EmbeddingGemma-256d";

pub struct OnnxEmbedder {
    session: Mutex<Session>,
    tokenizer: Tokenizer,
    outputs_pooled: bool,
    pub accelerator: String,
}

fn providers() -> Vec<(&'static str, ExecutionProviderDispatch)> {
    let mut v: Vec<(&'static str, ExecutionProviderDispatch)> = vec![];
    #[cfg(any(windows, target_os = "linux"))]
    v.push(("CUDA", ort::ep::CUDA::default().build()));
    #[cfg(windows)]
    v.push(("DirectML", ort::ep::DirectML::default().build()));
    #[cfg(target_os = "macos")]
    v.push(("CoreML", ort::ep::CoreML::default().build()));
    v.push(("CPU", CPU::default().build()));
    v
}

impl OnnxEmbedder {
    pub fn try_load(dir: &Path) -> Option<Self> {
        let model = ["model.onnx", "model_quantized.onnx", "model_q4.onnx", "model_fp16.onnx"]
            .iter()
            .map(|f| dir.join(f))
            .find(|p| p.exists())?;
        let tok_path = dir.join("tokenizer.json");
        if !tok_path.exists() {
            return None;
        }
        let mut tokenizer = Tokenizer::from_file(&tok_path).ok()?;
        tokenizer
            .with_truncation(Some(TruncationParams { max_length: MAX_TOKENS, ..Default::default() }))
            .ok()?;
        tokenizer.with_padding(Some(PaddingParams::default()));

        let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        // Try the full provider list first; if a GPU provider breaks session creation, retry CPU-only.
        let attempts: Vec<Vec<(&str, ExecutionProviderDispatch)>> = vec![
            providers(),
            vec![("CPU", CPU::default().build())],
        ];
        for list in attempts {
            let label = list.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(" > ");
            let eps: Vec<ExecutionProviderDispatch> = list.into_iter().map(|(_, e)| e).collect();
            let built = (|| -> ort::Result<Session> {
                Ok(Session::builder()?
                    .with_execution_providers(eps)?
                    .with_intra_threads(threads)?
                    .commit_from_file(&model)?)
            })();
            if let Ok(session) = built {
                let pooled = session.outputs().iter().any(|o| o.name() == "sentence_embedding");
                return Some(OnnxEmbedder {
                    session: Mutex::new(session),
                    tokenizer,
                    outputs_pooled: pooled,
                    accelerator: label,
                });
            }
        }
        None
    }

    fn run(&self, texts: Vec<String>) -> Vec<Vec<f32>> {
        let zero = || vec![vec![0f32; DIM]; 0];
        let n = texts.len();
        if n == 0 {
            return zero();
        }
        let Ok(enc) = self.tokenizer.encode_batch(texts, true) else {
            return vec![vec![0.0; DIM]; n];
        };
        let len = enc[0].get_ids().len();
        let ids: Vec<i64> = enc.iter().flat_map(|e| e.get_ids().iter().map(|&x| x as i64)).collect();
        let mask: Vec<i64> = enc.iter().flat_map(|e| e.get_attention_mask().iter().map(|&x| x as i64)).collect();
        let fail = || vec![vec![0.0; DIM]; n];

        let (Ok(t_ids), Ok(t_mask)) = (
            Tensor::from_array(([n, len], ids)),
            Tensor::from_array(([n, len], mask.clone())),
        ) else {
            return fail();
        };
        let mut session = self.session.lock();
        let Ok(outputs) = session.run(ort::inputs!["input_ids" => t_ids, "attention_mask" => t_mask]) else {
            return fail();
        };
        let key = if self.outputs_pooled { "sentence_embedding" } else { "last_hidden_state" };
        let Ok((shape, data)) = outputs[key].try_extract_tensor::<f32>() else {
            return fail();
        };
        let mut result = Vec::with_capacity(n);
        if self.outputs_pooled {
            let d = shape[shape.len() - 1] as usize;
            for i in 0..n {
                result.push(finish(&data[i * d..(i + 1) * d]));
            }
        } else {
            // mean-pool the token embeddings over the attention mask
            let d = shape[2] as usize;
            for i in 0..n {
                let mut acc = vec![0f32; d];
                let mut cnt = 0f32;
                for t in 0..len {
                    if mask[i * len + t] == 1 {
                        let row = &data[(i * len + t) * d..(i * len + t + 1) * d];
                        acc.iter_mut().zip(row).for_each(|(a, b)| *a += b);
                        cnt += 1.0;
                    }
                }
                acc.iter_mut().for_each(|a| *a /= cnt.max(1.0));
                result.push(finish(&acc));
            }
        }
        result
    }
}

/// Matryoshka truncation to 256 dims, then re-normalise.
fn finish(full: &[f32]) -> Vec<f32> {
    let mut v: Vec<f32> = full.iter().take(DIM).copied().collect();
    normalize(&mut v);
    v
}

impl Embedder for OnnxEmbedder {
    fn name(&self) -> &str {
        NAME
    }
    fn accelerator(&self) -> String {
        self.accelerator.clone()
    }
    fn embed_docs(&self, texts: &[String]) -> Vec<Vec<f32>> {
        self.run(texts.iter().map(|t| format!("title: none | text: {t}")).collect())
    }
    fn embed_query(&self, text: &str) -> Vec<f32> {
        self.run(vec![format!("task: search result | query: {text}")]).remove(0)
    }
}
