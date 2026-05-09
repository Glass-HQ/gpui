use std::{fs, path::Path, path::PathBuf};

use anyhow::{Context, Result};

use crate::{cli::Platform, ios};

#[derive(Clone, Debug)]
pub struct BuildOptions {
    pub sim: bool,
    pub release: bool,
    pub device: Option<String>,
}

#[derive(Clone, Debug)]
pub struct HostPaths {
    pub workspace_root: PathBuf,
    pub host_root: PathBuf,
}

pub trait Host {
    fn sync(&self) -> Result<()>;
    fn list_devices(&self) -> Result<()>;
    fn build(&self, options: &BuildOptions) -> Result<()>;
    fn run(&self, demo: Option<&str>, options: &BuildOptions) -> Result<()>;
    fn build_rust(&self) -> Result<()>;
}

pub enum LoadedHost {
    Ios(ios::IosHost),
}

impl LoadedHost {
    pub fn load(platform: Platform) -> Result<Self> {
        match platform {
            Platform::Ios => Ok(Self::Ios(ios::IosHost::load()?)),
        }
    }

    pub fn sync(&self) -> Result<()> {
        match self {
            Self::Ios(host) => host.sync(),
        }
    }

    pub fn list_devices(&self) -> Result<()> {
        match self {
            Self::Ios(host) => host.list_devices(),
        }
    }

    pub fn build(&self, options: &BuildOptions) -> Result<()> {
        match self {
            Self::Ios(host) => {
                host.build(options)?;
                Ok(())
            }
        }
    }

    pub fn run(&self, demo: Option<&str>, options: &BuildOptions) -> Result<()> {
        match self {
            Self::Ios(host) => host.run(demo, options),
        }
    }

    pub fn build_rust(&self) -> Result<()> {
        match self {
            Self::Ios(host) => host.build_rust(),
        }
    }
}

pub fn workspace_root() -> Result<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .map(|path| path.to_path_buf())
        .context("failed to resolve workspace root")
}

pub fn remove_generated_path(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    if path.is_dir() {
        fs::remove_dir_all(path)
            .with_context(|| format!("failed to remove generated directory {}", path.display()))?;
    } else {
        fs::remove_file(path)
            .with_context(|| format!("failed to remove generated file {}", path.display()))?;
    }

    Ok(())
}
