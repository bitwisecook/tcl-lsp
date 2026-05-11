# CLI installation guide

The `tcl` and `f5` command-line tools are self-contained Python
zipapps.  Pick one of the methods below.

## One-line installer

```sh
curl -fsSL https://raw.githubusercontent.com/bitwisecook/tcl-lsp/main/scripts/install.sh | sh
```

Works on macOS, Debian/Ubuntu, RHEL/CentOS/Rocky/Alma, Fedora,
Arch/Manjaro, and Alpine.  Needs Python 3.10+ — the script installs it
through the system package manager if it's missing.

To inspect the script before running it:

```sh
curl -fsSLo install.sh https://raw.githubusercontent.com/bitwisecook/tcl-lsp/main/scripts/install.sh
less install.sh
sh install.sh --version
sh install.sh
```

For fully unattended runs (skips every prompt, including PATH/rc edits):

```sh
curl -fsSL https://raw.githubusercontent.com/bitwisecook/tcl-lsp/main/scripts/install.sh \
  | TCL_LSP_ASSUME_YES=1 sh
```

The installer can also be re-run to update an existing install — it
detects the prior copy on `$PATH` and offers to update it in place.

## Manual install

Download the `.pyz` artefact(s) from
[GitHub Releases](https://github.com/bitwisecook/tcl-lsp/releases/latest),
move into a directory on your `$PATH`, and mark executable.

```sh
# Replace <version> with the version from the release page.
curl -fLO https://github.com/bitwisecook/tcl-lsp/releases/latest/download/tcl-<version>.pyz
curl -fLO https://github.com/bitwisecook/tcl-lsp/releases/latest/download/f5-<version>.pyz

# Move into a directory that's already on your $PATH, e.g.:
mv tcl-<version>.pyz ~/.local/bin/tcl
mv f5-<version>.pyz  ~/.local/bin/f5
chmod +x ~/.local/bin/tcl ~/.local/bin/f5
```

`~/.local/bin` is the standard XDG user-bin and is on `$PATH` by
default on most distros.  Use `/usr/local/bin` (with `sudo mv`) for a
system-wide install, or any other directory you've added to `$PATH`.

Verify:

```sh
tcl --help
f5  --help
```

Python 3.10+ must be on the host.  Install via `brew install python@3.14`
on macOS or your system package manager on Linux (`apt install python3`
on Debian/Ubuntu, `dnf install python3` on Fedora, etc.).

## Verifying downloads

Each release ships a `SHA256SUMS` file (and, for releases built by the
post-2026-05 CI, a `SHA256SUMS.cosign.bundle` keyless OIDC signature
covering it).  To verify a manual download:

```sh
tag="v1.9.0"
curl -fLO "https://github.com/bitwisecook/tcl-lsp/releases/download/$tag/SHA256SUMS"

# `--ignore-missing` skips entries for assets you haven't downloaded —
# `SHA256SUMS` covers every release artefact, but you usually only have
# the one or two you actually installed.
sha256sum --ignore-missing -c SHA256SUMS 2>/dev/null \
    || shasum -a 256 --ignore-missing -c SHA256SUMS

# Optional cosign signature check
curl -fLO "https://github.com/bitwisecook/tcl-lsp/releases/download/$tag/SHA256SUMS.cosign.bundle"
cosign verify-blob \
    --bundle SHA256SUMS.cosign.bundle \
    --certificate-identity-regexp "^https://github.com/bitwisecook/tcl-lsp/\.github/workflows/.+@refs/tags/" \
    --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
    SHA256SUMS
```

The one-line installer does both checks automatically when the files
are present.

## Updating

Re-run the same one-liner.  The installer detects the existing copy
on `$PATH`, asks once whether to update it in place, and overwrites
only the zipapp.  Falls back to the install picker if the existing
file isn't one of ours.

## Uninstall

```sh
rm -f ~/.local/bin/tcl ~/.local/bin/f5 ~/.local/bin/tcl-lsp-mcp-server.pyz
# Optional: shell completion + AI integrations
rm -f ~/.local/share/bash-completion/completions/{tcl,f5}
rm -f "${ZDOTDIR:-$HOME}/.zsh/completions/_tcl" "${ZDOTDIR:-$HOME}/.zsh/completions/_f5"
rm -f ~/.config/fish/completions/{tcl,f5}.fish
rm -rf ~/.claude/skills/{irule,tcl,tk}-* ~/.claude/tcl-ai.pyz ~/.claude/prompts
claude mcp remove tcl-lsp 2>/dev/null || true
```

## Installer environment variables

`install.sh --help` lists everything.  Most installs need none.  The
common ones:

| Variable | Effect |
|----------|--------|
| `TCL_LSP_PREFIX` | Install directory (default: prompt; non-interactive: `~/.local/bin`). |
| `TCL_LSP_ONLY` | `tcl`, `f5`, or `both` (default: both). |
| `TCL_LSP_VERSION` | Pin a release tag instead of `latest`. |
| `TCL_LSP_ASSUME_YES` / `TCL_LSP_ASSUME_NO` | Answer yes / no to every prompt. |
| `TCL_LSP_NO_DEPS` | Don't install Python / curl / wget / unzip / Tcl runtime deps. |
| `TCL_LSP_NO_VERIFY` | Install without SHA256SUMS verification (only do this if you trust the network path). |
| `TCL_LSP_REQUIRE_COSIGN` | Also require a cosign signature on SHA256SUMS (SHA256SUMS alone is required by default). |

## Building from source

```sh
git clone https://github.com/bitwisecook/tcl-lsp
cd tcl-lsp
uv sync --extra dev
make zipapp-tcl zipapp-f5
# Output in build/tcl-<version>.pyz, build/f5-<version>.pyz
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full build matrix and
release procedure.
