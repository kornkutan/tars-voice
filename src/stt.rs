use anyhow::{anyhow, Context, Result};
use std::path::PathBuf;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct Stt {
    ctx: WhisperContext,
}

pub fn model_path(model: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home)
        .join("Library")
        .join("Caches")
        .join("tars-voice")
        .join("whisper")
        .join(format!("ggml-{model}.bin"))
}

pub fn ensure_model(model: &str) -> Result<PathBuf> {
    let path = model_path(model);
    if path.exists() {
        return Ok(path);
    }
    std::fs::create_dir_all(path.parent().unwrap())?;
    let url = format!(
        "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-{model}.bin"
    );
    eprintln!("[tars-voice] downloading whisper model {model} (first run only)...");
    let tmp = path.with_extension("bin.part");
    let status = std::process::Command::new("curl")
        .args(["-L", "--fail", "--progress-bar", "-o"])
        .arg(&tmp)
        .arg(&url)
        .status()
        .context("failed to run curl")?;
    if !status.success() {
        let _ = std::fs::remove_file(&tmp);
        anyhow::bail!("model download failed for {model} from {url}");
    }
    std::fs::rename(&tmp, &path)?;
    Ok(path)
}

impl Stt {
    pub fn load(model: &str) -> Result<Stt> {
        let path = ensure_model(model)?;
        let ctx = WhisperContext::new_with_params(
            path.to_str().unwrap_or_default(),
            WhisperContextParameters::new(),
        )
        .map_err(|e| anyhow!("failed to load whisper model {model}: {e:?}"))?;
        Ok(Stt { ctx })
    }

    pub fn transcribe(&self, samples: &[f32], language: &str) -> Result<String> {
        if samples.is_empty() {
            return Ok(String::new());
        }
        let mut state = self.ctx.create_state().map_err(|e| anyhow!("{e:?}"))?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        let threads = std::thread::available_parallelism()
            .map(|n| n.get() as i32)
            .unwrap_or(4)
            .min(8);
        params.set_n_threads(threads);
        params.set_translate(false);
        params.set_no_context(true);
        match language {
            "" | "auto" => {
                params.set_language(None);
            }
            lang => {
                params.set_language(Some(lang));
            }
        }
        state
            .full(params, samples)
            .map_err(|e| anyhow!("whisper inference failed: {e:?}"))?;
        let n = state.full_n_segments().map_err(|e| anyhow!("{e:?}"))?;
        let mut text = String::new();
        for i in 0..n {
            if let Ok(seg) = state.full_get_segment_text_lossy(i) {
                text.push_str(&seg);
            }
        }
        Ok(text.trim().to_string())
    }
}
