#!/usr/bin/env bash
# Install the project's git hooks into the repo's hooks directory.
#
# Currently installs:
#   pre-push  — refuses to push unless 'make check-all' (or 'make test-slow',
#               which is a strict superset) has been run against the current
#               worktree.  See scripts/hooks/pre-push.
#
# Resolves the destination via `git rev-parse --git-path hooks` so it works
# correctly for git worktrees (where .git is a file) and for repos that set
# core.hooksPath.
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
src_dir="$repo_root/scripts/hooks"
dst_dir=$(cd "$repo_root" && git rev-parse --git-path hooks)
case "$dst_dir" in
    /*) ;;
    *)  dst_dir="$repo_root/$dst_dir" ;;
esac

mkdir -p "$dst_dir"

for hook in pre-push; do
    src="$src_dir/$hook"
    dst="$dst_dir/$hook"
    if [ ! -f "$src" ]; then
        echo "install-hooks: missing source hook $src" >&2
        exit 1
    fi
    install -m 0755 "$src" "$dst"
    echo "==> Installed $hook -> $dst"
done

chmod +x "$repo_root/scripts/worktree-fingerprint.sh"
cat <<EOF
==> Hooks installed.  Before pushing:
==>   make check-all   (full lint + typecheck for every language → tmp/check-all.stamp)
==> Before opening a PR:
==>   make test-slow   (everything; writes tmp/check-all.stamp + tmp/test-slow.stamp)
EOF
