use std::{fs, io, path::Path};

use anyhow::{Context, Result, bail};

/// Return false for missing directories and refuse symlinks in output paths.
fn is_output_dir(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() || !meta.is_dir() => {
            bail!(
                "output directory is a symlink or not a directory: {}",
                path.display()
            );
        }
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("checking {}", path.display())),
    }
}

pub(crate) fn clear_target(name: &str, base: &Path) -> Result<()> {
    let output = base.join("targets");
    let target_dir = output.join(name);
    if is_output_dir(&output)? && is_output_dir(&target_dir)? {
        for entry in fs::read_dir(&target_dir)
            .with_context(|| format!("reading {}", target_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            let result = if entry.file_type()?.is_dir() {
                fs::remove_dir_all(&path)
            } else {
                // This also removes symlinks without following them.
                fs::remove_file(&path)
            };
            result.with_context(|| format!("removing {}", path.display()))?;
        }
    }
    println!("{name}: cleared");
    Ok(())
}
