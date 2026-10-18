use std::{fs, path::Path, process::Command};

fn write(root: &Path, path: &str, contents: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn clear(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_heritage"))
        .arg("--config")
        .arg(root.join("config.toml"))
        .arg("--clear")
        .args(args)
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap()
}

fn config(root: &Path) {
    write(
        root,
        "config.toml",
        "[target.one]\ninclude = ['../gone']\n[target.two]\ninclude = []\n[target.three]\ninclude = []\n",
    );
}

#[test]
fn clear_all_empties_configured_targets_without_copying_or_touching_other_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    config(root);
    write(root, "source.txt", "source");
    write(root, "targets/one/nested/deep/file", "old");
    write(root, "targets/one/.hidden", "hidden");
    write(root, "targets/two/file", "old");
    write(root, "targets/unconfigured/file", "keep");
    write(root, "targets/.keep", "keep");
    let output = clear(root, &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for target in ["one", "two"] {
        assert_eq!(
            fs::read_dir(root.join("targets").join(target))
                .unwrap()
                .count(),
            0
        );
    }
    assert!(!root.join("targets/three").exists());
    assert_eq!(
        fs::read_to_string(root.join("source.txt")).unwrap(),
        "source"
    );
    assert!(root.join("targets/unconfigured/file").is_file());
    assert!(root.join("targets/.keep").is_file());
}

#[test]
fn repeated_target_filters_clear_only_selected_targets() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    config(root);
    for name in ["one", "two", "three"] {
        write(root, &format!("targets/{name}/file"), "old");
    }
    let output = clear(root, &["--target", "one", "-t", "three", "--no-progress"]);
    assert!(output.status.success());
    assert!(!root.join("targets/one/file").exists());
    assert!(!root.join("targets/three/file").exists());
    assert_eq!(
        fs::read_to_string(root.join("targets/two/file")).unwrap(),
        "old"
    );
}

#[test]
fn missing_targets_are_a_repeatable_noop() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    config(root);
    for _ in 0..2 {
        assert!(clear(root, &[]).status.success());
    }
    assert!(!root.join("targets").exists());
}

#[test]
fn invalid_selections_and_target_names_fail_before_deleting_anything() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    config(root);
    write(root, "targets/one/file", "keep");
    let output = clear(root, &["--target", "one", "--target", "unknown"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown target"));
    assert!(root.join("targets/one/file").is_file());
    write(
        root,
        "config.toml",
        "[target.one]\ninclude = []\n[target.'../escape']\ninclude = []\n",
    );
    let output = clear(root, &[]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid target name"));
    assert!(root.join("targets/one/file").is_file());
}

#[cfg(unix)]
#[test]
fn clearing_unlinks_child_symlinks_without_touching_their_targets() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let external = tempfile::tempdir().unwrap();
    config(root);
    write(external.path(), "file", "keep");
    write(root, "targets/one/nested/file", "remove");
    symlink(external.path(), root.join("targets/one/nested/dir-link")).unwrap();
    symlink(
        external.path().join("file"),
        root.join("targets/one/file-link"),
    )
    .unwrap();
    symlink(
        external.path().join("absent"),
        root.join("targets/one/dangling"),
    )
    .unwrap();
    assert!(clear(root, &["-t", "one"]).status.success());
    assert_eq!(fs::read_dir(root.join("targets/one")).unwrap().count(), 0);
    assert_eq!(
        fs::read_to_string(external.path().join("file")).unwrap(),
        "keep"
    );
}

#[cfg(unix)]
#[test]
fn clearing_rejects_symlinked_output_directories() {
    use std::os::unix::fs::symlink;
    for symlink_parent in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let external = tempfile::tempdir().unwrap();
        config(root);
        write(external.path(), "file", "keep");
        write(external.path(), "one/file", "keep");
        let link = if symlink_parent {
            root.join("targets")
        } else {
            fs::create_dir(root.join("targets")).unwrap();
            root.join("targets/one")
        };
        symlink(external.path(), &link).unwrap();
        let output = clear(root, &["-t", "one"]);
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("symlink"));
        assert!(fs::symlink_metadata(link).unwrap().file_type().is_symlink());
        assert_eq!(
            fs::read_to_string(external.path().join("file")).unwrap(),
            "keep"
        );
        assert_eq!(
            fs::read_to_string(external.path().join("one/file")).unwrap(),
            "keep"
        );
    }
}
