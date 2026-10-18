use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::matching::{Rules, validate_patterns};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
    pub(crate) target: BTreeMap<String, Target>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Target {
    pub(crate) include: Vec<String>,
    #[serde(default)]
    pub(crate) exclude: Vec<String>,
    pub(crate) size_limit_kb: Option<u64>,
    pub(crate) file_size_limit_kb: Option<u64>,
}

impl Config {
    pub(crate) fn load(path: &Path) -> Result<Self> {
        toml::from_str(&fs::read_to_string(path)?)
            .with_context(|| format!("parsing {}", path.display()))
    }

    pub(crate) fn validate_targets(&self, selected: &[String]) -> Result<()> {
        if self.target.is_empty() {
            bail!("config must contain at least one [target.NAME] section");
        }
        for name in selected {
            if !self.target.contains_key(name) {
                bail!("unknown target {name:?}");
            }
        }
        for name in self.target.keys() {
            if !valid_target_name(name) {
                bail!("invalid target name {name:?}: expected a single directory name");
            }
        }
        Ok(())
    }

    pub(crate) fn validate_sources(&self, base: &Path) -> Result<()> {
        for target in self.target.values() {
            validate_patterns(&target.include)?;
            validate_patterns(&target.exclude)?;
            limit_bytes(target.file_size_limit_kb)?;
            limit_bytes(target.size_limit_kb)?;
            Rules::new(&target.include, base)?;
            Rules::new(&target.exclude, base)?;
        }
        Ok(())
    }
}

pub(crate) fn limit_bytes(value: Option<u64>) -> Result<u64> {
    value
        .map(|kb| {
            kb.checked_mul(1024)
                .context("size limit in KB is too large")
        })
        .transpose()
        .map(|limit| limit.unwrap_or(u64::MAX))
}

fn valid_target_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains(['/', '\\'])
        && !name.contains('\0')
}
