#!/bin/sh

set -eu

check_clean() {
    changes=$(git status --porcelain --untracked-files=all --ignored -- "$@")
    if [ -n "$changes" ]; then
        echo 'error: template files have local changes or untracked files; commit or move them first' >&2
        printf '%s\n' "$changes" >&2
        return 1
    fi
}

# Keep the body in a function so updating this script is safe while it runs.
main() {
    root=$(git rev-parse --show-toplevel)
    cd "$root"

    # These paths belong to the template; private data stays outside this list.
    set -- Cargo.toml Cargo.lock config_example.toml justfile rustfmt.toml \
        README.md src tests .github .vscode tools

    check_clean "$@"

    if ! git remote get-url upstream >/dev/null 2>&1; then
        git remote add upstream https://github.com/lesterrry/heritage.git
        git remote set-url --push upstream DISABLED
    fi

    git fetch --no-tags upstream master
    revision=$(git rev-parse FETCH_HEAD)
    check_clean "$@"
    for path in "$@"; do
        tracked=$(git ls-files -- "$path")
        # A removed top-level path may already be absent from both repositories.
        if git cat-file -e "$revision:$path" 2>/dev/null || [ -n "$tracked" ]; then
            git restore --source="$revision" --worktree -- "$path"
        fi
    done

    printf 'Template files updated from %s. Review and commit the changes when ready.\n' "$revision"
    git status --short --untracked-files=all -- "$@"
}

main
