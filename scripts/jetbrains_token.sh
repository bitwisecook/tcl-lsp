#!/usr/bin/env bash
# jetbrains_token.sh — resolve the JetBrains Marketplace upload token.
#
# Looks for the token in three places, in order:
#
#   1. $JETBRAINS_TOKEN environment variable (CI / one-off override)
#   2. macOS Keychain (service "tcl-lsp-jetbrains", account "publish")
#   3. Linux libsecret / gnome-keyring (service "tcl-lsp-jetbrains",
#      attribute account=publish)
#
# Prints the token to stdout on success. Exits 1 with a hint on failure.
#
# Store the token once with:
#
#   macOS:
#     security add-generic-password -s tcl-lsp-jetbrains \
#         -a publish -w '<TOKEN>' -U
#
#   Linux (needs libsecret-tools):
#     printf '<TOKEN>' | secret-tool store \
#         --label='JetBrains Marketplace publish token (tcl-lsp)' \
#         service tcl-lsp-jetbrains account publish

set -uo pipefail

SERVICE="tcl-lsp-jetbrains"
ACCOUNT="publish"

if [ -n "${JETBRAINS_TOKEN:-}" ]; then
    printf '%s' "$JETBRAINS_TOKEN"
    exit 0
fi

case "$(uname -s)" in
    Darwin)
        if command -v security >/dev/null 2>&1; then
            if token="$(security find-generic-password \
                    -s "$SERVICE" -a "$ACCOUNT" -w 2>/dev/null)"; then
                printf '%s' "$token"
                exit 0
            fi
        fi
        ;;
    Linux)
        if command -v secret-tool >/dev/null 2>&1; then
            if token="$(secret-tool lookup \
                    service "$SERVICE" account "$ACCOUNT" 2>/dev/null)"; then
                if [ -n "$token" ]; then
                    printf '%s' "$token"
                    exit 0
                fi
            fi
        fi
        ;;
esac

cat >&2 <<EOF
error: could not resolve JetBrains Marketplace token.

Tried, in order:
  1. \$JETBRAINS_TOKEN environment variable
  2. macOS Keychain     (service=$SERVICE, account=$ACCOUNT)
  3. Linux libsecret    (service=$SERVICE, account=$ACCOUNT)

Store it once with:

  macOS:
    security add-generic-password -s $SERVICE -a $ACCOUNT -w '<TOKEN>' -U

  Linux (apt install libsecret-tools):
    printf '<TOKEN>' | secret-tool store \\
        --label='JetBrains Marketplace publish token (tcl-lsp)' \\
        service $SERVICE account $ACCOUNT

Get a token from https://plugins.jetbrains.com/author/me/tokens
(scope: Plugin uploader).
EOF
exit 1
