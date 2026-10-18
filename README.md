# heritage

Copy files selected by TOML rules into `targets/<name>/`, with size limits.

```sh
cargo run --release
cargo run --release -- --config /path/to/config.toml
cargo run --release -- --target default
cargo run --release -- --target default --target docs --no-progress
cargo run --release -- --clear
cargo run --release -- --clear --target default --target docs
cargo run -- --help
```

Running without arguments reads `config.toml`. Copy `config_example.toml` to
`config.toml` and adjust its paths first.

```toml
[target.default]
size_limit_kb = 2048
file_size_limit_kb = 128
include = ["src", "Cargo.*", "/absolute/path/to/documents/**/*.txt"]
exclude = ["node_modules", "*.woff", "secret*", "!secret-public.txt"]
```

- Each `[target.NAME]` defines a separate copy operation. By default all targets
  run, in name order. Use `--target NAME` to select targets.
- `--clear` removes all contents (including hidden files and nested directories)
  of the configured target directories instead of copying files. The same
  `--config` and repeated `--target NAME` options apply. Existing target directories
  are left empty; missing directories are left absent. Other directories under
  `targets/` are untouched. Source patterns and size limits do not affect clearing.
  Symlinks inside a target are removed without following them; symlinks used as
  `targets/` or as a target directory are rejected.
- Relative patterns and the output `targets/` directory are relative to the config
  file's directory, regardless of the working directory used to launch the app.
  Absolute patterns match paths from the filesystem root.
- Both lists use [gitignore syntax](https://docs.rs/ignore/latest/ignore/gitignore/struct.GitignoreBuilder.html):
  `*`, `**`, `?`, character classes, directory patterns, comments, and `!` negation.
  Later rules in a list override earlier rules. A matching include selects a file;
  a matching exclude then removes it. A bare name matches at any depth, including
  in external source directories (for example, `*.woff` excludes external fonts). Including
  a directory includes its descendants. Excluded directories are pruned, so their
  children cannot be re-included without first re-including the directory.
  Patterns must not contain `.` or `..` path components; `~` is not expanded.
  Repository `.gitignore` files are not read implicitly. Hidden files are included
  when they match. Symlinks are not followed or copied.
- Files inside the config directory retain their path relative to that directory:
  `src/main.rs` becomes `targets/default/src/main.rs`. External files retain their
  absolute path with the initial `/` removed: `/Users/me/report.txt` becomes
  `targets/default/Users/me/report.txt`. Conflicting destination paths are reported
  as errors. Generated `targets/` contents are always omitted from scans.
- `file_size_limit_kb` caps each file; `size_limit_kb` caps the sum of bytes copied
  during that target's current run. Both are optional (unlimited if omitted).
  One KB is 1024 bytes, and a file exactly at the limit is accepted. Zero allows
  only empty files. Files run in source path order; files that exceed either limit
  emit a warning and are skipped, and smaller later files can still fit.
- Matching destination files are replaced using staged copies. Existing files
  that are excluded, absent from the source, or skipped remain in the destination.
  The total limit concerns the current copy operation, not these retained files.
  File permissions are preserved; timestamps and empty directories are not copied.
  Directory symlinks in output paths are rejected.
- Warnings about size limits do not fail the command. Config, scan, and copy errors
  exit with a nonzero status. Each completed target prints copied bytes and file
  counts. Use `--no-progress` to disable animation; progress is also hidden when
  stderr is not a terminal.

Development checks:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

To update a private repository created from this template:

```sh
just update-template
```

The command fetches `master` from the `upstream` remote and replaces the shared
files with that snapshot. If `upstream` is absent, it is added as
`https://github.com/lesterrry/heritage.git` with pushing disabled. An existing
`upstream` remote is used as configured; `origin` is unchanged.

The managed paths are `Cargo.toml`, `Cargo.lock`, `config_example.toml`, `justfile`,
`rustfmt.toml`, `README.md`, `src/`, `tests/`, `.github/`, `.vscode/`, and `tools/`.
These directories belong entirely to the template; files removed from the template
are also removed locally. The local `config.toml`, `targets/`, `.gitignore`, and
other paths are preserved. Avoid storing private files inside managed directories.

Local changes, untracked files, or ignored files inside managed paths stop the
update before fetching. Changes to backup data outside those paths do not block
it. Review the resulting changes with `git diff` and `git status`, then commit
them yourself. The command does not stage, commit, or push changes, and does not
merge the unrelated template histories. This recipe uses syntax supported by
`just 1.21.0`.

For an existing private repository that does not have this recipe yet, first
publish these changes to the public template. With any existing edits to the
shared files committed, bootstrap the recipe in the private repository once:

```sh
git fetch --no-tags https://github.com/lesterrry/heritage.git master
git restore --source=FETCH_HEAD --worktree -- justfile tools/update-template.sh
git add justfile tools/update-template.sh
git commit -m "Add template update command"
just update-template
```
