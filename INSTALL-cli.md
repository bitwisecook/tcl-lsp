# CLI installation guide

Step-by-step instructions for installing the `tcl` and `f5` command-line
tools on **macOS** (Homebrew), **Debian/Ubuntu**,
**RHEL/CentOS/Rocky/Alma**, **Fedora**, **Arch/Manjaro**, and **Alpine**.

Both CLIs are distributed as self-contained Python
[zipapps](https://docs.python.org/3/library/zipapp.html) (`.pyz`) — no
`pip install`, no virtualenv, no Rust toolchain.  They need only a
Python 3.10+ interpreter on the host.

| CLI | Artefact | Purpose |
|-----|----------|---------|
| `tcl` | `tcl-<version>.pyz` | Unified Tcl tools: format, lint, optimise, diff, bytecode, WASM, `pkg`, `venv`, `help` |
| `f5`  | `f5-<version>.pyz`  | F5 BIG-IP tools: `cleanup`, `grep`, `diff`, `redact`, `pcap-remap`, `irule …`, `tmsh`, `fetch`, … |

For editor / LSP server installation see [INSTALL-editors.md](INSTALL-editors.md).

---

## Quickest path — one-line installer

The installer detects the host OS (macOS, Debian/Ubuntu, Fedora/RHEL,
Arch, Alpine) and the login shell (bash, zsh, fish), installs Python
3.10+ through the native package manager when missing, scans `PATH`
for a writable install directory (prompts you to pick between
`~/.local/bin`, `~/bin`, `/usr/local/bin`, `/opt/homebrew/bin`, and
any user-owned directory already on `PATH`), downloads the latest
`tcl` and `f5` zipapps there, and offers to wire up `PATH` and shell
completion.

When the chosen location is not user-writable (e.g. `/usr/local/bin`
under a non-root user) the installer **asks explicitly** before
escalating to `sudo` for the file copy.  The prompt defaults to **no**
— declining aborts the install with a hint to re-run with a different
`TCL_LSP_PREFIX`.  Set `TCL_LSP_ASSUME_YES=1` to opt in to sudo
non-interactively.  Python package installs go through `sudo`/`doas`
separately and have their own confirmation gate.

If the `claude` or `codex` CLI is present (or `~/.claude` / `~/.codex`
exists), the installer also offers to:

- download `tcl-lsp-mcp-server-<version>.pyz` from the same release and
  register it with each detected client (`claude mcp add tcl-lsp …`
  for Claude Code; an `[mcp_servers.tcl_lsp]` block in
  `~/.codex/config.toml` for Codex);
- download `tcl-lsp-claude-skills-<version>.zip` and extract the 26
  skill directories, the analysis zipapp, and the prompt bundle into
  `~/.claude/` (so `/irule-fix`, `/tcl-explain`, etc. become available
  inside Claude Code).

In non-interactive use (`curl … | sh`), the location prompt is
skipped and `$TCL_LSP_PREFIX` (default `~/.local/bin`) is used.

```sh
curl -fsSL https://github.com/bitwisecook/tcl-lsp/releases/latest/download/install.sh | sh
```

Prefer not to pipe straight into `sh`?  Download and read it first:

```sh
curl -fsSLo install.sh https://github.com/bitwisecook/tcl-lsp/releases/latest/download/install.sh
less install.sh
sh install.sh
```

### Interactive vs non-interactive

Each prompt has its own default tuned to whether silently doing the
thing is safe:

| Prompt | Interactive default | Piped (`curl … | sh`) default | Opt-out |
|--------|---------------------|-------------------------------|---------|
| Choose CLIs (`tcl`, `f5`) | both | both | `TCL_LSP_ONLY=tcl` or `=f5` |
| Choose install location | `$PREFIX` (currently `~/.local/bin`) | `$PREFIX` (no prompt) | `TCL_LSP_PREFIX=…` |
| Update existing install in place (when detected) | **yes** | **yes** | answer no, or `TCL_LSP_ASSUME_NO=1` |
| Use sudo to write to a non-writable directory | **no** | **no** (install aborts) | `TCL_LSP_ASSUME_YES=1` |
| Add `$PREFIX` to PATH (modifies rc file) | **yes** | **no** | `TCL_LSP_NO_PATH=1` |
| Install shell completion | **yes** | **no** | `TCL_LSP_NO_COMP=1` |
| Install MCP server (detected AI client) | **yes** | **yes** | `TCL_LSP_NO_MCP=1` |
| Install Claude Code skills (detected) | **yes** | **yes** | `TCL_LSP_NO_SKILLS=1` |

When the installer is run interactively and either `whiptail` (preinstalled
on Debian/Ubuntu/RHEL) or `dialog` is on `PATH`, the prompts switch to a
TUI: the CLI picker is a checklist, the install-location picker is a
menu, and yes/no questions are arrow-key-driven dialogs.  Set
`TCL_LSP_NO_TUI=1` to keep the plain-text prompts.  In non-interactive
mode (piped stdin) the TUI is never used.

### CLI runtime dependencies

After the `tcl` / `f5` zipapps land, the installer surveys the
external tools those CLIs shell out to.  The CLIs themselves are
self-contained Python zipapps; these extras only affect specific
verbs.

| Dependency | Required for | Used by |
|------------|--------------|---------|
| `tclsh` (any of 8.5 / 8.6 / 9.0) | `tcl pkg`, `tcl venv`, `tcl explore` against a real interpreter | `tcl` |
| `openssh` (`ssh`, `scp`) | `f5 fetch` over SSH to a BIG-IP | `f5` |
| `sshpass` | `f5 fetch` with password auth (optional fallback to key auth) | `f5` |
| `tshark` (Wireshark CLI) | `f5 explain-flow --tshark`, `f5 enrich-pcapng`, `f5 pcap-remap` libpcap input | `f5` |

The installer reports any that are missing and asks once
(default **no**) whether to install the lot via the OS package
manager.  Set `TCL_LSP_NO_DEPS=1` to skip the prompt entirely.

The survey runs after `install_cli`, so what's offered depends on
which CLIs you installed: install `tcl` only and you only get the
`tclsh` prompt; install `f5` only and you skip `tclsh`.

Skipping is fine — every CLI verb that needs an external tool detects
its absence at runtime and prints a clear message (`ssh not found`,
`tshark not available`, etc.).

Package names per distro:

|         | Debian/Ubuntu | RHEL/Fedora | Arch | Alpine | macOS (brew) |
|---------|---------------|-------------|------|--------|--------------|
| `tclsh` | `tcl` | `tcl` | `tcl` | `tcl` | `tcl-tk` |
| `ssh`   | `openssh-client` | `openssh-clients` | `openssh` | `openssh-client` | preinstalled |
| `sshpass` | `sshpass` | `sshpass` | `sshpass` | `sshpass` | `sshpass` |
| `tshark` | `tshark` | `wireshark-cli` | `wireshark-cli` | `tshark` | `wireshark` |

### Updating an existing install

Before prompting for an install location, the installer scans `$PATH`
for an existing `tcl` / `f5` and runs a two-tier zipapp identity check:

1. **Cheap fingerprint** — file starts with a `python` shebang and the
   first 2 KB contains the ZIP local-file-header signature `PK\x03\x04`.
2. **Deep peek** — opens the ZIP and looks for a tcl-lsp marker
   (`lsp/_build_info.py` in the namelist, or one of our known module
   imports like `explorer.tcl_cli` / `ai.mcp.tcl_mcp_server` /
   `lsp.server` inside `__main__.py`).

The deep peek tries `unzip` first (fast), falls back to a
`python3 -c "import zipfile…"` snippet (Python is required anyway), and
finally falls back to a raw `grep -aql 'lsp/_build_info.py'` against
the file (the central directory stores filenames uncompressed, so the
literal path appears verbatim regardless of compression).

If a file looks like one of our zipapps, the installer prompts:

```
==> found existing tcl at /usr/local/bin/tcl
Update existing tcl at /usr/local/bin/tcl (in place)? [Y/n]
```

Saying yes pins `$PREFIX` to that directory and skips the install-location
picker.  Saying no falls back to the normal picker.  If a file is on
`PATH` but doesn't look like one of our zipapps the installer warns and
runs the picker — it never overwrites unrelated binaries.

### Naming conflicts

After the install location is locked in, the installer scans `$PATH`
and your shell rc file again to surface conflicts that would shadow
the new binary:

- another `tcl` / `f5` earlier on `$PATH` (whether it's a prior
  tcl-lsp install at a different location or an unrelated tool);
- a shell `alias` / `abbr` in `$HOME/.{bashrc,bash_profile,zshrc,profile}`
  or `$XDG_CONFIG_HOME/fish/config.fish` that would intercept the name.

When either is found you get three options:

| Choice | Effect |
|--------|--------|
| `keep`   | Install with the original name.  The existing shadow stays in place — you'll need to fix it (remove the alias, reorder `$PATH`) to actually use the new binary. |
| `rename` | Install our binaries with a `-lsp` suffix (`tcl-lsp`, `f5-lsp`).  Shell completion is skipped because the bundled completion script registers handlers for the original name. |
| `abort`  | Cancel the install. |

For non-interactive runs (`curl … | sh`), set `TCL_LSP_SUFFIX=-lsp` to
pick "rename" up front.  Otherwise the conflict warning is printed and
the install proceeds with the original name.

### Overwriting an existing file at the install location

When the target path (e.g. `~/.local/bin/tcl`) already exists and
*doesn't* look like one of our zipapps, the installer refuses to
clobber it without confirmation:

```
warn: ~/.local/bin/tcl already exists and is not a tcl-lsp zipapp
warn: (no Python shebang or ZIP signature in first 2KB)
Overwrite ~/.local/bin/tcl anyway? [y/N]
```

The prompt defaults to **no** — declining aborts the install with a
hint to remove the file or choose a different `TCL_LSP_PREFIX`.

The asymmetry is deliberate: rc-file and completion-directory
mutation get an explicit yes from the user, while the AI integrations
that only kick in when a client is already present are opt-out (a
detected `claude` / `codex` install is treated as consent for the
matching extras).

`TCL_LSP_ASSUME_YES=1` forces yes on every prompt; `TCL_LSP_ASSUME_NO=1`
forces no on every prompt.

```sh
# Unattended install of everything, including rc-file edit + completion:
curl -fsSL https://github.com/bitwisecook/tcl-lsp/releases/latest/download/install.sh \
  | TCL_LSP_ASSUME_YES=1 sh
```

### Installer environment overrides

| Variable | Default | Effect |
|----------|---------|--------|
| `TCL_LSP_VERSION` | `latest` | Pin a release tag (e.g. `v1.2.3`). |
| `TCL_LSP_PREFIX`  | *prompt* (default `$HOME/.local/bin`) | Install directory. Set to bypass the interactive picker. |
| `TCL_LSP_ONLY`    | `both` | `tcl`, `f5`, or `both`. |
| `TCL_LSP_NO_DEPS` | unset | Skip every package-manager install — Python (required), curl/wget, unzip (for skills), and the CLI runtime dep batch.  Fails loudly when a required dep is missing. |
| `TCL_LSP_NO_PATH` | unset | Do not modify the shell rc file. |
| `TCL_LSP_NO_COMP` | unset | Skip shell completion install. |
| `TCL_LSP_NO_MCP`    | unset | Skip MCP server install for AI clients. |
| `TCL_LSP_NO_SKILLS` | unset | Skip Claude Code skills install. |
| `TCL_LSP_NO_CLAUDE` | unset | Ignore Claude Code even if detected. |
| `TCL_LSP_NO_CODEX`  | unset | Ignore Codex even if detected. |
| `TCL_LSP_ASSUME_YES` | unset | Answer "yes" to every prompt (required for unattended PATH / completion install). |
| `TCL_LSP_ASSUME_NO`  | unset | Answer "no" to every prompt (skip rc and completion entirely). |
| `TCL_LSP_REPO`    | `bitwisecook/tcl-lsp` | Source repository. Non-default values print a warning. |
| `TCL_LSP_OS`      | auto-detected | Bypass `/etc/os-release` detection: `debian`, `rhel`, `fedora`, `arch`, `alpine`, or `macos`. Useful in containers or hosts where `/etc/os-release` is locked down. |
| `TCL_LSP_NO_VERIFY`      | unset | Skip the SHA256SUMS verification step entirely. |
| `TCL_LSP_REQUIRE_VERIFY` | unset | Fail the install if the release has no `SHA256SUMS` file. Default behaviour is to warn and continue. |
| `TCL_LSP_NO_TUI` | unset | Force plain-text prompts even when `whiptail` or `dialog` is on PATH. |
| `TCL_LSP_SUFFIX` | unset  | Suffix to append to installed binary names (e.g. `-lsp` → `tcl-lsp`, `f5-lsp`). Used to avoid clashing with an existing `tcl` / `f5` on PATH. |

Example — install only `f5`, system-wide, fully automated:

```sh
curl -fsSL https://github.com/bitwisecook/tcl-lsp/releases/latest/download/install.sh \
  | sudo TCL_LSP_ONLY=f5 TCL_LSP_PREFIX=/usr/local/bin TCL_LSP_ASSUME_YES=1 sh
```

---

## Integrity verification

Each release publishes a `SHA256SUMS` file listing the SHA-256 hash of
every artefact, plus an optional `SHA256SUMS.cosign.bundle` with a
cosign keyless signature bound to the
[GitHub Actions OIDC identity](https://docs.github.com/en/actions/deployment/security-hardening-your-deployments/about-security-hardening-with-openid-connect)
of the publish workflow.

The installer automatically downloads `SHA256SUMS` and verifies every
artefact it installs.  When `cosign` is available on the host and the
release ships the bundle, the signature is verified against the
workflow identity.

If `SHA256SUMS` is missing from a release (e.g. an older release that
predates this feature), the installer prints a warning and proceeds.
Set `TCL_LSP_REQUIRE_VERIFY=1` to fail instead.

### Manual verification

```sh
tag="v1.9.0"
mkdir -p /tmp/verify && cd /tmp/verify

# Pull SUMS and the artefact(s) you intend to install
curl -fLO "https://github.com/bitwisecook/tcl-lsp/releases/download/$tag/SHA256SUMS"
curl -fLO "https://github.com/bitwisecook/tcl-lsp/releases/download/$tag/tcl-${tag#v}.pyz"
curl -fLO "https://github.com/bitwisecook/tcl-lsp/releases/download/$tag/f5-${tag#v}.pyz"

# Verify hashes
sha256sum -c SHA256SUMS 2>/dev/null || shasum -a 256 -c SHA256SUMS

# (Optional) verify the cosign signature
curl -fLO "https://github.com/bitwisecook/tcl-lsp/releases/download/$tag/SHA256SUMS.cosign.bundle" 2>/dev/null \
  && cosign verify-blob \
       --bundle SHA256SUMS.cosign.bundle \
       --certificate-identity-regexp "^https://github.com/bitwisecook/tcl-lsp/\.github/workflows/.+@refs/tags/" \
       --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
       SHA256SUMS
```

---

## Python prerequisite

Both CLIs require **Python 3.10 or newer**.  Python 3.14 is recommended.

### macOS — Homebrew

```sh
brew install python@3.14
```

Homebrew installs the interpreter at `/opt/homebrew/bin/python3` (Apple
Silicon) or `/usr/local/bin/python3` (Intel).

### Debian / Ubuntu

```sh
sudo apt-get update
sudo apt-get install -y python3 ca-certificates curl
```

Ubuntu 22.04 LTS and newer ship Python 3.10+ by default.  On older
releases use the
[deadsnakes PPA](https://launchpad.net/~deadsnakes/+archive/ubuntu/ppa):

```sh
sudo add-apt-repository ppa:deadsnakes/ppa
sudo apt-get update
sudo apt-get install -y python3.14
```

### RHEL / CentOS Stream / Rocky / Alma

```sh
sudo dnf install -y python3 ca-certificates curl
```

RHEL 9 / Rocky 9 / Alma 9 ship Python 3.9 as `python3` — install a
newer interpreter alongside:

```sh
sudo dnf install -y python3.11
```

The installer script picks the highest `python3.10+` it can find.

### Fedora

```sh
sudo dnf install -y python3 ca-certificates curl
```

Fedora 37+ already ships 3.10+.

### Arch / Manjaro

```sh
sudo pacman -Sy python ca-certificates curl
```

### Alpine

```sh
sudo apk add --no-cache python3 ca-certificates curl
```

### Verify

```sh
python3 --version   # 3.10 or newer
```

---

## Manual install

If you'd rather not run the installer script, fetch the zipapps
directly from [GitHub Releases](https://github.com/bitwisecook/tcl-lsp/releases).

### Per-user install (no sudo)

```sh
mkdir -p ~/.local/bin

curl -L -o ~/.local/bin/tcl \
  https://github.com/bitwisecook/tcl-lsp/releases/latest/download/tcl-<version>.pyz
chmod +x ~/.local/bin/tcl

curl -L -o ~/.local/bin/f5 \
  https://github.com/bitwisecook/tcl-lsp/releases/latest/download/f5-<version>.pyz
chmod +x ~/.local/bin/f5
```

Replace `<version>` with the version from the
[latest release page](https://github.com/bitwisecook/tcl-lsp/releases/latest).

Ensure `~/.local/bin` is on `PATH`:

```sh
# bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc

# zsh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc

# fish
fish_add_path ~/.local/bin
```

### System-wide install

Drop the same files into `/usr/local/bin/` instead, and `chmod 755`
them:

```sh
sudo install -m 0755 ~/Downloads/tcl-<version>.pyz /usr/local/bin/tcl
sudo install -m 0755 ~/Downloads/f5-<version>.pyz  /usr/local/bin/f5
```

The downloaded `.pyz` is self-contained — no `pip install`, no
virtualenv.  If you'd rather not rely on the embedded shebang, invoke
explicitly:

```sh
python3 /usr/local/bin/tcl --help
python3 /usr/local/bin/f5 cleanup samples/bigip/bigip.conf
```

### Verify

```sh
tcl --help   # prints: tcl <verb> [options] [inputs...]
f5  --help   # prints: f5  <verb> [options] [inputs...]
```

---

## Shell completion

Both CLIs ship a `completion <shell>` verb that prints a ready-to-install
completion script.  Run it **after** the binary is on `PATH`.

### bash

```sh
# Per-user
mkdir -p ~/.local/share/bash-completion/completions
tcl completion bash > ~/.local/share/bash-completion/completions/tcl
f5  completion bash > ~/.local/share/bash-completion/completions/f5

# Or eagerly source from ~/.bashrc:
#   source <(tcl completion bash)
#   source <(f5  completion bash)
```

On macOS you may need to install
[bash-completion](https://github.com/scop/bash-completion):

```sh
brew install bash-completion@2
```

### zsh

```sh
mkdir -p "${ZDOTDIR:-$HOME}/.zsh/completions"
tcl completion zsh > "${ZDOTDIR:-$HOME}/.zsh/completions/_tcl"
f5  completion zsh > "${ZDOTDIR:-$HOME}/.zsh/completions/_f5"
```

Then ensure `~/.zshrc` contains, **before** `compinit`:

```sh
fpath=("${ZDOTDIR:-$HOME}/.zsh/completions" $fpath)
autoload -Uz compinit && compinit
```

For a system-wide zsh install, write straight to `site-functions`:

```sh
sudo sh -c 'tcl completion zsh > /usr/share/zsh/site-functions/_tcl'
sudo sh -c 'f5  completion zsh > /usr/share/zsh/site-functions/_f5'
```

### fish

```sh
mkdir -p ~/.config/fish/completions
tcl completion fish > ~/.config/fish/completions/tcl.fish
f5  completion fish > ~/.config/fish/completions/f5.fish
```

Then start a new fish session.

### Hints

Pass `--hint` to print install instructions to stderr alongside the
script:

```sh
tcl completion bash --hint
f5  completion zsh  --hint
```

Completion covers verb names, every flag, dialect choices, optimiser
profiles, `pkg` / `venv` / `docker` actions, `*.conf` / `*.scf`
positional paths, and Tcl/iRules source paths (`*.tcl`, `*.tk`,
`*.itcl`, `*.tm`, `*.irul`, `*.irule`, `*.iapp`, `*.iappimpl`).

---

## AI client integration

The installer detects two AI clients:

| Client | Detected when | What gets installed |
|--------|---------------|---------------------|
| **Claude Code** | `claude` is on PATH **or** `~/.claude/` exists | MCP server (`claude mcp add tcl-lsp -- python3 …`) and the skills bundle (26 `/irule-*`, `/tcl-*`, `/tk-*` slash commands under `~/.claude/skills/`) |
| **Codex** | `codex` is on PATH **or** `~/.codex/` exists | MCP server registered as `[mcp_servers.tcl_lsp]` in `~/.codex/config.toml` (existing file is backed up with a timestamp suffix before append) |

The MCP server zipapp lands in `$PREFIX/tcl-lsp-mcp-server.pyz` (same
directory as `tcl` and `f5`).  Re-running the installer is safe — the
Claude Code registration is `remove`-then-`add`, the Codex block is
skipped if `[mcp_servers.tcl_lsp]` is already present, and the skills
extract overwrites only the `skills/`, `prompts/`, and `tcl-ai.pyz`
entries it ships.

### Manual MCP install

If you'd rather wire things up by hand, download
`tcl-lsp-mcp-server-<version>.pyz` from
[GitHub Releases](https://github.com/bitwisecook/tcl-lsp/releases) and
register it:

**Claude Code**:

```sh
claude mcp add tcl-lsp -- python3 ~/.local/bin/tcl-lsp-mcp-server.pyz
```

**Codex** — add to `~/.codex/config.toml`:

```toml
[mcp_servers.tcl_lsp]
command = "python3"
args = ["/home/you/.local/bin/tcl-lsp-mcp-server.pyz"]
```

### Manual Claude Code skills install

```sh
curl -L -o /tmp/skills.zip \
  https://github.com/bitwisecook/tcl-lsp/releases/latest/download/tcl-lsp-claude-skills-<version>.zip
unzip /tmp/skills.zip -d /tmp/skills
cp -R /tmp/skills/tcl-lsp-claude-skills-*/{skills,prompts} ~/.claude/
cp    /tmp/skills/tcl-lsp-claude-skills-*/tcl-ai.pyz ~/.claude/
```

The `SKILL.md` files inside the zip already reference
`~/.claude/tcl-ai.pyz` and `~/.claude/prompts/` — no further wiring
needed.

---

## Updating

To pick up a new release, re-run the installer (it overwrites the
binaries in place):

```sh
curl -fsSL https://github.com/bitwisecook/tcl-lsp/releases/latest/download/install.sh | sh
```

Or, for a manual install, replace the file in `~/.local/bin` (or
`/usr/local/bin`) with the new release artefact.

---

## Uninstall

```sh
rm -f ~/.local/bin/tcl ~/.local/bin/f5 ~/.local/bin/tcl-lsp-mcp-server.pyz
rm -f ~/.local/share/bash-completion/completions/{tcl,f5}
rm -f "${ZDOTDIR:-$HOME}/.zsh/completions/_tcl" "${ZDOTDIR:-$HOME}/.zsh/completions/_f5"
rm -f ~/.config/fish/completions/{tcl,f5}.fish

# Claude Code skills and MCP registration
rm -rf ~/.claude/skills/{irule,tcl,tk}-*
rm -f  ~/.claude/tcl-ai.pyz
rm -rf ~/.claude/prompts
claude mcp remove tcl-lsp 2>/dev/null || true

# Codex MCP registration — edit ~/.codex/config.toml and delete the
# [mcp_servers.tcl_lsp] block.
```

For a system-wide install, swap the paths to `/usr/local/bin/...` and
`/usr/share/zsh/site-functions/...`.

---

## Building from source

For development, clone the repository and run from source:

```sh
git clone https://github.com/bitwisecook/tcl-lsp
cd tcl-lsp
uv sync --extra dev

python -m explorer.tcl_cli lint samples/
python -m explorer.f5_cli  cleanup samples/bigip/bigip.conf
```

Or build a fresh zipapp:

```sh
make zipapp-tcl   # build/tcl-<version>.pyz
make zipapp-f5    # build/f5-<version>.pyz
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full build matrix.

---

## Troubleshooting

**`tcl: command not found` after install**
Run `echo $PATH` and check that `~/.local/bin` (or whatever you set
`TCL_LSP_PREFIX` to) is listed.  Open a new shell after the installer
edits your rc file.

**`bad interpreter: /usr/bin/env: python3.10`**
The bundled shebang assumes `python3` resolves to 3.10+.  If your
system `python3` is older, invoke explicitly:

```sh
python3.14 ~/.local/bin/tcl --help
```

Or symlink a newer interpreter into `PATH` ahead of the system one.

**Corporate proxy / private GitHub Enterprise**
Set `https_proxy` and `http_proxy` before running the installer, or
download the artefact through your browser and run the manual install
above.  Override the source repo with `TCL_LSP_REPO=org/fork`.

**`SSL: CERTIFICATE_VERIFY_FAILED` on macOS**
Run `/Applications/Python\ 3.14/Install\ Certificates.command` after
installing python.org's interpreter, or use the Homebrew interpreter
which uses the system trust store.
