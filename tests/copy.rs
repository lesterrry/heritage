use std::{fs, path::Path, process::Command};

fn write(root: &Path, path: &str, contents: impl AsRef<[u8]>) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn run(root: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_heritage"))
        .args([
            "--config",
            root.join("config.toml").to_str().unwrap(),
            "--no-progress",
        ])
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap()
}

fn fixture() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

#[test]
fn matching_overwrite_and_repeat_run() {
    let dir = fixture();
    let root = dir.path().canonicalize().unwrap();
    write(&root, "src/a.txt", "new");
    write(&root, "src/.hidden", "hidden");
    write(&root, "src/no.woff", "font");
    write(&root, "src/node_modules/package/file.txt", "dependency");
    write(&root, "src/drop.txt", "excluded");
    write(&root, "src/drop-keep.txt", "keep");
    write(&root, "targets/default/src/a.txt", "old, longer contents");
    write(
        &root,
        "config.toml",
        r#"
[target.default]
include = ["**", "src", "src/a.txt"]
exclude = ["config.toml", "node_modules", "*.woff", "drop*", "!drop-keep.txt"]
"#,
    );
    for _ in 0..2 {
        let output = run(&root);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("copied 3 files"));
    }
    assert_eq!(
        fs::read(root.join("targets/default/src/a.txt")).unwrap(),
        b"new"
    );
    assert!(root.join("targets/default/src/.hidden").is_file());
    assert!(root.join("targets/default/src/drop-keep.txt").is_file());
    assert!(!root.join("targets/default/src/drop.txt").exists());
    assert!(!root.join("targets/default/src/no.woff").exists());
    assert!(!root.join("targets/default/src/node_modules").exists());
    assert!(!root.join("targets/default/targets").exists());
}

#[test]
fn size_limits_skip_large_files_but_keep_trying_smaller_ones() {
    let dir = fixture();
    let root = dir.path();
    write(root, "data/a", vec![0; 1025]);
    write(root, "data/b", vec![1; 1024]);
    write(root, "data/c", vec![2; 1024]);
    write(root, "data/d", vec![3; 512]);
    write(
        root,
        "config.toml",
        r#"
[target.default]
include = ["data"]
file_size_limit_kb = 1
size_limit_kb = 1
"#,
    );
    let output = run(root);
    assert!(output.status.success());
    assert_eq!(
        fs::read(root.join("targets/default/data/b")).unwrap().len(),
        1024
    );
    for file in ["a", "c", "d"] {
        assert!(!root.join("targets/default/data").join(file).exists());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("exceeds file limit"));
    assert!(stderr.contains("remaining target budget"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("skipped 3"));
}

#[test]
fn absolute_patterns_globs_negation_and_target_selection() {
    let dir = fixture();
    let root = dir.path().canonicalize().unwrap();
    let external = fixture();
    let external_root = external.path().canonicalize().unwrap();
    write(&external_root, "folder/a.txt", "a");
    write(&external_root, "folder/b.txt", "b");
    write(&external_root, "folder/c.jpg", "c");
    write(&external_root, "folder/node_modules/dependency.txt", "dep");
    let include = format!("{}/folder/**", external_root.display());
    let exclude = format!("{}/folder/b.txt", external_root.display());
    write(
        &root,
        "config.toml",
        format!(
            r#"
[target.one]
include = ["{include}"]
exclude = ["{exclude}", "*.jpg", "node_modules"]
[target.two]
include = ["missing"]
"#
        ),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_heritage"))
        .args([
            "-c",
            root.join("config.toml").to_str().unwrap(),
            "-t",
            "one",
            "--no-progress",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let destination = root
        .join("targets/one")
        .join(external_root.strip_prefix("/").unwrap())
        .join("folder");
    assert_eq!(fs::read(destination.join("a.txt")).unwrap(), b"a");
    assert!(!destination.join("b.txt").exists());
    assert!(!destination.join("c.jpg").exists());
    assert!(!destination.join("node_modules").exists());
    assert!(!root.join("targets/two").exists());
}

#[test]
fn include_negation_and_zero_limit() {
    let dir = fixture();
    let root = dir.path();
    write(root, "data/a.txt", "a");
    write(root, "data/b.txt", "b");
    write(root, "data/empty", "");
    write(
        root,
        "config.toml",
        r#"
[target.default]
include = ["data", "!data/a.txt"]
size_limit_kb = 0
"#,
    );
    let output = run(root);
    assert!(output.status.success());
    assert!(root.join("targets/default/data/empty").is_file());
    assert!(!root.join("targets/default/data/a.txt").exists());
    assert!(!root.join("targets/default/data/b.txt").exists());
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("copied 1 files (0 bytes), skipped 1")
    );
}

#[test]
fn invalid_config_fails_before_creating_targets() {
    let dir = fixture();
    let root = dir.path();
    write(root, "config.toml", "[target.'../escape']\ninclude = []\n");
    let output = run(root);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid target name"));
    assert!(!root.join("targets").exists());
}

#[cfg(unix)]
#[test]
fn source_symlinks_are_not_followed_and_output_symlinks_are_rejected() {
    use std::os::unix::fs::symlink;
    let dir = fixture();
    let root = dir.path();
    write(root, "data/file", "source");
    symlink(root.join("data/file"), root.join("data/link")).unwrap();
    write(
        root,
        "config.toml",
        "[target.default]\ninclude = ['data']\n",
    );
    assert!(run(root).status.success());
    assert!(!root.join("targets/default/data/link").exists());
    let outside = fixture();
    fs::remove_dir_all(root.join("targets/default/data")).unwrap();
    symlink(outside.path(), root.join("targets/default/data")).unwrap();
    let output = run(root);
    assert!(!output.status.success());
    assert!(!outside.path().join("file").exists());
}

#[test]
fn smaller_files_can_fit_after_a_budget_skip() {
    let dir = fixture();
    let root = dir.path();
    write(root, "data/a", vec![0; 800]);
    write(root, "data/b", vec![0; 900]);
    write(root, "data/c", vec![0; 200]);
    write(
        root,
        "config.toml",
        "[target.default]\ninclude = ['data']\nsize_limit_kb = 1\n",
    );
    let output = run(root);
    assert!(output.status.success());
    assert!(root.join("targets/default/data/a").is_file());
    assert!(!root.join("targets/default/data/b").exists());
    assert!(root.join("targets/default/data/c").is_file());
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("copied 2 files (1000 bytes), skipped 1")
    );
}
