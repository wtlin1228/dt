use anyhow::{self, bail};
use std::path::{Path, PathBuf};

pub trait ToCanonicalString {
    fn to_canonical_string(&self) -> anyhow::Result<String>;
}

impl ToCanonicalString for PathBuf {
    fn to_canonical_string(&self) -> anyhow::Result<String> {
        Ok(self.canonicalize()?.to_string_lossy().to_string())
    }
}

#[derive(Debug)]
pub struct PathResolver {
    base_url: String,
}

impl PathResolver {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    pub fn resolve_path(&self, current_path: &str, import_src: &str) -> anyhow::Result<String> {
        let p = match import_src.starts_with(".") {
            true => Path::new(current_path).with_file_name(import_src),
            false => Path::new(&self.base_url).join(import_src),
        };

        if let Ok(resolved_path) = p.join("index.js").canonicalize() {
            return Ok(resolved_path.to_string_lossy().to_string());
        }

        if let Ok(resolved_path) = p.join("index.ts").canonicalize() {
            return Ok(resolved_path.to_string_lossy().to_string());
        }

        for extension in ["ts", "tsx", "js", "jsx"] {
            let mut p = p.clone();
            p.set_extension(extension);
            if let Ok(resolved_path) = p.canonicalize() {
                return Ok(resolved_path.to_string_lossy().to_string());
            }
        }

        bail!("Fail to resolve the import src {:?}", import_src)
    }
}
