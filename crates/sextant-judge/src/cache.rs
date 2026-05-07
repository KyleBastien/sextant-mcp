//! Content-addressed JSON cache for judge results.
//!
//! Keys are blake3 of `(provider, model, prompt)` rendered as hex. Files
//! land at `<dir>/<key>.json`. A cache hit short-circuits a network call,
//! which is critical both for cost and for the agent inner-loop UX.

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::JudgeResult;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("io ({path:?}): {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("malformed cache entry ({path:?}): {source}")]
    Decode {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

pub struct Cache {
    dir: PathBuf,
}

impl Cache {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn key(provider: &str, model: &str, prompt: &str) -> String {
        let mut h = blake3::Hasher::new();
        h.update(provider.as_bytes());
        h.update(b"\0");
        h.update(model.as_bytes());
        h.update(b"\0");
        h.update(prompt.as_bytes());
        h.finalize().to_hex().to_string()
    }

    fn path_for(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{key}.json"))
    }

    pub fn get(&self, key: &str) -> Result<Option<JudgeResult>, CacheError> {
        let path = self.path_for(key);
        let raw = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(CacheError::Io { path, source }),
        };
        let res =
            serde_json::from_slice(&raw).map_err(|source| CacheError::Decode { path, source })?;
        Ok(Some(res))
    }

    pub fn put(&self, key: &str, value: &JudgeResult) -> Result<(), CacheError> {
        ensure_dir(&self.dir)?;
        let path = self.path_for(key);
        let json = serde_json::to_vec(value).map_err(|source| CacheError::Decode {
            path: path.clone(),
            source,
        })?;
        std::fs::write(&path, json).map_err(|source| CacheError::Io { path, source })?;
        Ok(())
    }
}

fn ensure_dir(dir: &Path) -> Result<(), CacheError> {
    if dir.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(dir).map_err(|source| CacheError::Io {
        path: dir.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JudgeFinding, JudgeSeverity};

    fn sample() -> JudgeResult {
        JudgeResult {
            findings: vec![JudgeFinding {
                severity: JudgeSeverity::Warn,
                message: "x".into(),
                line: Some(1),
                end_line: None,
            }],
        }
    }

    #[test]
    fn key_changes_with_each_input() {
        let a = Cache::key("anthropic", "m", "p");
        let b = Cache::key("openai", "m", "p");
        let c = Cache::key("anthropic", "m2", "p");
        let d = Cache::key("anthropic", "m", "p2");
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }

    #[test]
    fn miss_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::new(dir.path().to_path_buf());
        assert!(cache.get("nothing").unwrap().is_none());
    }

    #[test]
    fn put_then_get_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::new(dir.path().join("nested"));
        let key = Cache::key("p", "m", "prompt");
        cache.put(&key, &sample()).unwrap();
        let hit = cache.get(&key).unwrap().expect("hit");
        assert_eq!(hit, sample());
    }
}
