//! Storage in config.
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

/// Locate code files
///
/// + cache -> the path to cache
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Storage {
    cache: String,
    code: String,
    root: String,
    scripts: Option<String>,
    #[serde(default = "default_notes")]
    notes: String,
}

fn default_notes() -> String {
    "notes".into()
}

impl Default for Storage {
    fn default() -> Self {
        Self {
            cache: "Problems".into(),
            code: "code".into(),
            scripts: Some("scripts".into()),
            root: "~/.leetcode".into(),
            notes: "notes".into(),
        }
    }
}

impl Storage {
    /// convert root path
    pub fn root(&self) -> Result<String> {
        let home = dirs::home_dir()
            .ok_or(Error::NoneError)?
            .to_string_lossy()
            .to_string();
        let path = self.root.replace('~', &home);
        Ok(path)
    }

    fn resolve_path(&self, value: &str) -> Result<PathBuf> {
        if value.starts_with('/') {
            Ok(PathBuf::from(value))
        } else if value.starts_with('~') {
            let home = dirs::home_dir()
                .ok_or(Error::NoneError)?
                .to_string_lossy()
                .to_string();
            Ok(PathBuf::from(value.replacen('~', &home, 1)))
        } else {
            Ok(PathBuf::from(self.root()?).join(value))
        }
    }

    fn ensure_dir(p: &PathBuf) -> Result<()> {
        if !p.exists() {
            fs::DirBuilder::new().recursive(true).create(p)?;
        }
        Ok(())
    }

    /// get cache path
    pub fn cache(&self) -> Result<String> {
        let root = PathBuf::from(self.root()?);
        if !root.exists() {
            info!("Generate cache dir at {:?}.", &root);
            fs::DirBuilder::new().recursive(true).create(&root)?;
        }

        Ok(root.join("Problems").to_string_lossy().to_string())
    }

    /// get code path
    pub fn code(&self) -> Result<String> {
        let p = self.resolve_path(&self.code)?;
        Self::ensure_dir(&p)?;
        Ok(p.to_string_lossy().to_string())
    }

    /// get scripts path
    pub fn scripts(mut self) -> Result<String> {
        if self.scripts.is_none() {
            self.scripts = Some("scripts".into());
        }
        let val = self.scripts.clone().ok_or(Error::NoneError)?;
        let p = self.resolve_path(&val)?;
        Self::ensure_dir(&p)?;
        Ok(p.to_string_lossy().to_string())
    }

    /// get notes path
    pub fn notes(&self) -> Result<String> {
        let p = self.resolve_path(&self.notes)?;
        Self::ensure_dir(&p)?;
        Ok(p.to_string_lossy().to_string())
    }
}
