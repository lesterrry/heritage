use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use walkdir::WalkDir;

struct Rule {
    root: PathBuf,
    matcher: Gitignore,
    external_basename: Option<Gitignore>,
}

pub(crate) struct Rules(Vec<Rule>);

impl Rules {
    pub(crate) fn new(patterns: &[String], base: &Path) -> Result<Self> {
        let mut rules = Vec::new();
        for pattern in patterns {
            let body = pattern.strip_prefix('!').unwrap_or(pattern);
            let root = if body.starts_with('/') {
                PathBuf::from("/")
            } else {
                base.to_path_buf()
            };
            let mut builder = GitignoreBuilder::new(&root);
            builder
                .add_line(None, pattern)
                .with_context(|| format!("invalid pattern {pattern:?}"))?;
            // Basename rules such as *.woff also apply to external absolute sources.
            let external_basename = if !body.trim_end_matches('/').contains('/') {
                let mut external = GitignoreBuilder::new("/");
                external.add_line(None, pattern)?;
                Some(external.build()?)
            } else {
                None
            };
            rules.push(Rule {
                root,
                matcher: builder.build()?,
                external_basename,
            });
        }
        Ok(Self(rules))
    }

    fn matches(&self, path: &Path, is_dir: bool) -> bool {
        let mut matched = false;
        for rule in &self.0 {
            let matcher = if path.starts_with(&rule.root) {
                Some(&rule.matcher)
            } else {
                rule.external_basename.as_ref()
            };
            if let Some(matcher) = matcher {
                let result = matcher.matched_path_or_any_parents(path, is_dir);
                if !result.is_none() {
                    matched = result.is_ignore();
                }
            }
        }
        matched
    }
}

/// Find the literal directory prefix so absolute includes don't scan the whole disk.
fn search_root(pattern: &str, base: &Path) -> Option<PathBuf> {
    if pattern.starts_with(['!', '#']) || pattern.trim().is_empty() {
        return None;
    }
    let pattern = pattern.trim_end();
    let absolute = pattern.starts_with('/');
    let mut root = if absolute {
        PathBuf::from("/")
    } else {
        base.to_path_buf()
    };
    // A bare name matches at any depth, just like a gitignore basename rule.
    if !absolute && !pattern.trim_end_matches('/').contains('/') {
        return Some(root);
    }
    for part in pattern.split('/').filter(|part| !part.is_empty()) {
        if part.contains(['*', '?', '[', '\\']) {
            break;
        }
        // Parent traversal isn't meaningful in gitignore rules. Reject it in validation.
        root.push(part);
    }
    Some(root)
}

pub(crate) fn validate_patterns(patterns: &[String]) -> Result<()> {
    for pattern in patterns {
        let body = pattern.strip_prefix('!').unwrap_or(pattern);
        if !body.starts_with('#') && body.split('/').any(|part| part == ".." || part == ".") {
            bail!("pattern {pattern:?}: use paths without '.' or '..' components");
        }
    }
    Ok(())
}

pub(crate) fn collect_files(
    include: &[String],
    exclude: &[String],
    base: &Path,
    output: &Path,
) -> Result<BTreeSet<PathBuf>> {
    let includes = Rules::new(include, base)?;
    let excludes = Rules::new(exclude, base)?;
    let roots: BTreeSet<_> = include
        .iter()
        .filter_map(|pattern| search_root(pattern, base))
        .collect();
    let mut files = BTreeSet::new();
    for root in &roots {
        if roots
            .iter()
            .any(|other| other != root && root.starts_with(other))
        {
            continue;
        }
        let walker = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                !entry.path().starts_with(output)
                    && !excludes.matches(entry.path(), entry.file_type().is_dir())
            });
        for entry in walker {
            let entry = entry.with_context(|| format!("scanning {}", root.display()))?;
            if entry.file_type().is_file() && includes.matches(entry.path(), false) {
                files.insert(entry.into_path());
            }
        }
    }
    Ok(files)
}
