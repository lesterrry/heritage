use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result, bail};
use indicatif::{ProgressBar, ProgressStyle};

use crate::{
    config::{Target, limit_bytes},
    matching::collect_files,
};

/// Refuse existing directory symlinks so copying stays inside the output tree.
fn create_output_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        create_output_dir(parent)?;
    }
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() || !meta.is_dir() => {
            bail!(
                "output directory is a symlink or not a directory: {}",
                path.display()
            );
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::create_dir(path).with_context(|| format!("creating {}", path.display()))
        }
        Err(error) => Err(error).with_context(|| format!("checking {}", path.display())),
    }
}

fn destination(source: &Path, base: &Path, target: &Path) -> PathBuf {
    let relative = source.strip_prefix(base).unwrap_or_else(|_| {
        source
            .strip_prefix(Path::new("/"))
            .expect("absolute source path")
    });
    target.join(relative)
}

pub(crate) fn copy_target(name: &str, target: &Target, base: &Path, progress: bool) -> Result<()> {
    let output = base.join("targets");
    let target_dir = output.join(name);
    let file_limit = limit_bytes(target.file_size_limit_kb)?;
    let total_limit = limit_bytes(target.size_limit_kb)?;
    let bar = if progress {
        ProgressBar::new_spinner()
    } else {
        ProgressBar::hidden()
    };
    bar.set_message(format!("{name}: scanning files"));
    bar.enable_steady_tick(Duration::from_millis(100));
    let files = collect_files(&target.include, &target.exclude, base, &output)?;
    let mut destinations = BTreeMap::new();
    for source in &files {
        let dest = destination(source, base, &target_dir);
        if let Some(previous) = destinations.insert(dest.clone(), source) {
            bail!(
                "{} and {} map to the same destination {}",
                previous.display(),
                source.display(),
                dest.display()
            );
        }
    }
    bar.set_style(ProgressStyle::with_template(
        "{spinner:.green} {prefix} [{bar:32.cyan/blue}] {pos}/{len} {msg}",
    )?);
    bar.set_prefix(name.to_owned());
    bar.set_length(files.len() as u64);
    create_output_dir(&target_dir)?;
    let mut copied = 0;
    let mut skipped = 0;
    let mut total = 0u64;
    for source in files {
        bar.set_message(source.display().to_string());
        let size = fs::metadata(&source)?.len();
        let remaining = total_limit - total;
        let reason = if size > file_limit {
            Some(format!("exceeds file limit of {file_limit} bytes"))
        } else if size > remaining {
            Some(format!(
                "exceeds remaining target budget of {remaining} bytes"
            ))
        } else {
            None
        };
        if let Some(reason) = reason {
            bar.suspend(|| {
                eprintln!(
                    "warning: {} ({size} bytes): {reason}; skipped",
                    source.display()
                )
            });
            skipped += 1;
            bar.inc(1);
            continue;
        }
        let dest = destination(&source, base, &target_dir);
        let parent = dest.parent().context("destination has no parent")?;
        create_output_dir(parent)?;
        // Stage the copy so failures and growing files don't corrupt existing backups.
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        let input = File::open(&source).with_context(|| format!("opening {}", source.display()))?;
        let allowance = file_limit.min(remaining);
        let bytes = io::copy(&mut input.take(allowance.saturating_add(1)), &mut temp)
            .with_context(|| format!("copying {}", source.display()))?;
        if bytes > allowance {
            bar.suspend(|| {
                eprintln!(
                    "warning: {} grew beyond the size limit while copying; skipped",
                    source.display()
                )
            });
            skipped += 1;
        } else {
            temp.as_file()
                .set_permissions(fs::metadata(&source)?.permissions())?;
            temp.persist(&dest)
                .with_context(|| format!("replacing {}", dest.display()))?;
            total += bytes;
            copied += 1;
        }
        bar.inc(1);
    }
    bar.finish_and_clear();
    println!("{name}: copied {copied} files ({total} bytes), skipped {skipped}");
    Ok(())
}
