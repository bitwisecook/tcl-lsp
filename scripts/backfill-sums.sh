#!/usr/bin/env bash
# Backfill SHA256SUMS (and optionally a cosign signature) onto an
# existing GitHub Release that predates the publish-checksums CI job.
#
# Usage:
#   scripts/backfill-sums.sh <tag> [--sign] [--upload] [--dry-run]
#
# Steps:
#   1. List the release's existing assets
#   2. Download each one (skipping any pre-existing SUMS / signature)
#   3. Cross-check the locally-computed sha256 against GitHub's
#      asset.digest field — fail if any disagree
#   4. Write SHA256SUMS in the canonical sorted form
#   5. (optional --sign) cosign sign-blob with the maintainer's
#      preferred identity (keyless OIDC on a workstation works fine)
#   6. (optional --upload) `gh release upload <tag> SHA256SUMS [bundle]
#      --clobber`
#
# Idempotent: re-running is a no-op when SUMS is already current.

set -euo pipefail

TAG="${1:-}"
SIGN=0
UPLOAD=0
DRY_RUN=0
shift || true
for arg in "$@"; do
    case "$arg" in
        --sign)    SIGN=1 ;;
        --upload)  UPLOAD=1 ;;
        --dry-run) DRY_RUN=1 ;;
        -h|--help) sed -n '2,20p' "$0" | sed 's/^# //; s/^#$//'; exit 0 ;;
        *) echo "unknown flag: $arg" >&2; exit 2 ;;
    esac
done

if [ -z "$TAG" ]; then
    echo "usage: $0 <tag> [--sign] [--upload] [--dry-run]" >&2
    exit 2
fi

REPO="${REPO:-bitwisecook/tcl-lsp}"

command -v gh >/dev/null || { echo "gh CLI required" >&2; exit 1; }
command -v sha256sum >/dev/null || command -v shasum >/dev/null \
    || { echo "sha256sum or shasum required" >&2; exit 1; }
if [ "$SIGN" = 1 ]; then
    command -v cosign >/dev/null || { echo "cosign required for --sign" >&2; exit 1; }
fi

workdir="$(mktemp -d -t tcl-lsp-backfill.XXXXXX)"
trap 'rm -rf -- "$workdir"' EXIT INT TERM

echo "==> backfilling SHA256SUMS for $REPO@$TAG (workdir: $workdir)"
cd "$workdir"

echo "==> listing release assets"
mapfile -t assets < <(gh release view "$TAG" --repo "$REPO" \
    --json assets --jq '.assets[].name' \
    | grep -Ev '^SHA256SUMS(\.[^/]+)?$')
if [ "${#assets[@]}" -eq 0 ]; then
    echo "no eligible assets found on $TAG" >&2
    exit 1
fi
printf '   %s\n' "${assets[@]}"

echo "==> downloading $(printf '%s\n' "${assets[@]}" | wc -l) asset(s)"
for name in "${assets[@]}"; do
    gh release download "$TAG" --repo "$REPO" --pattern "$name" --clobber
done

echo "==> computing SHA256SUMS"
if command -v sha256sum >/dev/null; then
    sha256sum -- "${assets[@]}" | LC_ALL=C sort -k2 > SHA256SUMS
else
    shasum -a 256 -- "${assets[@]}" | LC_ALL=C sort -k2 > SHA256SUMS
fi
cat SHA256SUMS

echo "==> cross-checking against GitHub-reported asset digests"
gh release view "$TAG" --repo "$REPO" --json assets \
    --jq '.assets[] | "\(.digest | sub("^sha256:";""))  \(.name)"' \
    | grep -Ev '  SHA256SUMS(\.[^/]+)?$' \
    | LC_ALL=C sort -k2 > expected
if ! diff -u expected SHA256SUMS; then
    echo "local hashes diverge from GitHub-reported digests — refusing to upload" >&2
    exit 1
fi
echo "   all hashes match"

if [ "$SIGN" = 1 ]; then
    # Cosign keyless OIDC produces a fresh bundle each time — if the
    # release already carries a valid one, re-signing would create a
    # divergent local artefact that nobody verifies against. Check the
    # release first; only sign when missing or explicitly forced via
    # --force-sign.
    existing_bundle=0
    if gh release view "$TAG" --repo "$REPO" --json assets \
         --jq '.assets[].name' 2>/dev/null \
         | grep -qx 'SHA256SUMS.cosign.bundle'; then
        existing_bundle=1
    fi
    if [ "$existing_bundle" = 1 ]; then
        echo "==> release already has SHA256SUMS.cosign.bundle — verifying:"
        gh release download "$TAG" --repo "$REPO" \
            --pattern 'SHA256SUMS.cosign.bundle' --clobber
        if cosign verify-blob \
             --bundle SHA256SUMS.cosign.bundle \
             --certificate-identity-regexp "^https://github.com/${REPO}/\.github/workflows/.+@refs/tags/" \
             --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
             SHA256SUMS >/dev/null 2>&1; then
            echo "   existing bundle verifies — not re-signing"
            SIGN=0
        else
            echo "   existing bundle does NOT verify — re-signing"
        fi
    fi
    if [ "$SIGN" = 1 ]; then
        echo "==> cosign sign-blob (keyless OIDC)"
        cosign sign-blob --yes --bundle SHA256SUMS.cosign.bundle SHA256SUMS
    fi
fi

if [ "$UPLOAD" = 1 ]; then
    if [ "$DRY_RUN" = 1 ]; then
        echo "==> [dry-run] gh release upload $TAG SHA256SUMS [SHA256SUMS.cosign.bundle] --clobber"
    else
        echo "==> uploading SHA256SUMS"
        gh release upload "$TAG" --repo "$REPO" SHA256SUMS --clobber
        if [ -f SHA256SUMS.cosign.bundle ]; then
            gh release upload "$TAG" --repo "$REPO" SHA256SUMS.cosign.bundle --clobber
        fi
    fi
else
    cat <<EOF

SHA256SUMS lives at: $workdir/SHA256SUMS
Upload with:
   gh release upload "$TAG" --repo "$REPO" "$workdir/SHA256SUMS" --clobber

(or re-run with --upload to do it now)
EOF
fi
