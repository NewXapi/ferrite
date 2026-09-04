//! Filesystem persistence for a single Agent run.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use harness_core::{AgentRun, AgentRunEvent};
use harness_tools::AgentToolResult;

/// Persistence errors.
#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("invalid path component `{0}`")]
    InvalidComponent(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

/// Filesystem store rooted at `<root>/<run-id>/`.
#[derive(Debug)]
pub struct RunPersistence {
    root: PathBuf,
    event_lock: Mutex<()>,
}

impl RunPersistence {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            event_lock: Mutex::new(()),
        }
    }

    pub fn run_dir(&self, run_id: &str) -> Result<PathBuf, PersistenceError> {
        Ok(self.root.join(validate_component(run_id)?))
    }

    pub async fn write_run(&self, run: &AgentRun) -> Result<PathBuf, PersistenceError> {
        let dir = self.ensure_run_dir(&run.id).await?;
        let path = dir.join("run.json");
        write_atomic_json(&path, run).await?;
        Ok(path)
    }

    pub async fn append_event(
        &self,
        run_id: &str,
        event: &AgentRunEvent,
    ) -> Result<(), PersistenceError> {
        let dir = self.ensure_run_dir(run_id).await?;
        let path = dir.join("events.jsonl");
        let line = serde_json::to_string(event)?;
        let _guard = self.event_lock.lock().await;
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;
        file.write_all(line.as_bytes()).await?;
        file.write_all(b"\n").await?;
        Ok(())
    }

    pub async fn write_tool_args(
        &self,
        run_id: &str,
        call_id: &str,
        args: &impl Serialize,
    ) -> Result<PathBuf, PersistenceError> {
        self.write_named_json(run_id, "tool-args", call_id, args)
            .await
    }

    pub async fn write_tool_result(
        &self,
        run_id: &str,
        result: &AgentToolResult,
    ) -> Result<PathBuf, PersistenceError> {
        self.write_named_json(run_id, "tool-results", &result.call_id, result)
            .await
    }

    pub async fn write_model_response(
        &self,
        run_id: &str,
        seq: u64,
        body: &impl Serialize,
    ) -> Result<PathBuf, PersistenceError> {
        self.write_named_json(run_id, "model-responses", &seq.to_string(), body)
            .await
    }

    pub async fn write_checkpoint(
        &self,
        run_id: &str,
        seq: u64,
        body: &impl Serialize,
    ) -> Result<PathBuf, PersistenceError> {
        self.write_named_json(run_id, "checkpoints", &seq.to_string(), body)
            .await
    }

    pub async fn load_run(&self, run_id: &str) -> Result<AgentRun, PersistenceError> {
        let path = self.run_dir(run_id)?.join("run.json");
        let bytes = tokio::fs::read(path).await?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub async fn load_events(&self, run_id: &str) -> Result<Vec<AgentRunEvent>, PersistenceError> {
        let path = self.run_dir(run_id)?.join("events.jsonl");
        let raw = match tokio::fs::read_to_string(path).await {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        raw.lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).map_err(PersistenceError::from))
            .collect()
    }

    async fn write_named_json(
        &self,
        run_id: &str,
        dir_name: &str,
        file_stem: &str,
        body: &impl Serialize,
    ) -> Result<PathBuf, PersistenceError> {
        let dir = self.ensure_run_dir(run_id).await?.join(dir_name);
        tokio::fs::create_dir_all(&dir).await?;
        let path = dir.join(format!("{}.json", validate_component(file_stem)?));
        write_atomic_json(&path, body).await?;
        Ok(path)
    }

    async fn ensure_run_dir(&self, run_id: &str) -> Result<PathBuf, PersistenceError> {
        let dir = self.run_dir(run_id)?;
        tokio::fs::create_dir_all(&dir).await?;
        Ok(dir)
    }
}

fn validate_component(value: &str) -> Result<&str, PersistenceError> {
    if value.is_empty()
        || value.contains('\0')
        || value.contains('/')
        || value.contains('\\')
        || value == "."
        || value == ".."
        || value.contains("..")
    {
        return Err(PersistenceError::InvalidComponent(value.to_string()));
    }
    Ok(value)
}

async fn write_atomic_json(path: &Path, body: &impl Serialize) -> Result<(), PersistenceError> {
    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
    let bytes = serde_json::to_vec_pretty(body)?;
    let suffix = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    let tmp = path.with_extension(format!("json.{suffix}.tmp"));
    tokio::fs::write(&tmp, bytes).await?;
    tokio::fs::rename(tmp, path).await?;
    Ok(())
}
