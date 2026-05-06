#!/usr/bin/env bash
# Install the project's git hooks into .git/hooks/.
#
# Currently installs:
#   pre-push  — refuses to push unless 'make test-slow' has been run against
#               the current worktree (see scripts/hooks/pre-push).
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
src_dir="$repo_root/scripts/hooks"
dst_dir="$repo_root/.git/hooks"

if [ ! -d "$dst_dir" ]; then
    echo "install-hooks: $dst_dir does not exist — is this a git checkout?" >&2
    exit 1
fi

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
