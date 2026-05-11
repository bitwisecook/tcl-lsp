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

When the chosen location is not user-writable (e.g. `/usr/local/bin`)
the installer escalates to `sudo` for the file copy only — Python
package installs always go through `sudo`/`doas` already.

In non-interactive use (`curl … | sh`), the location prompt is
skipped and `$TCL_LSP_PREFIX` (default `~/.local/bin`) is used.

```sh
curl -fsSL https://raw.githubusercontent.com/bitwisecook/tcl-lsp/main/scripts/install.sh | sh
```

Prefer not to pipe straight into `sh`?  Download and read it first:

```sh
curl -fsSLo install.sh https://raw.githubusercontent.com/bitwisecook/tcl-lsp/main/scripts/install.sh
less install.sh
sh install.sh
```

### Interactive vs non-interactive

When the installer is run from a real terminal it prompts before
modifying your shell rc file (PATH update) or installing shell
completion.  When piped (`curl … | sh`) it defaults prompts to **no**
so it never mutates dotfiles silently — it just drops the zipapps into
`$PREFIX` and prints what to add.

To opt in to the full unattended setup, set `TCL_LSP_ASSUME_YES=1`:

```sh
curl -fsSL https://raw.githubusercontent.com/bitwisecook/tcl-lsp/main/scripts/install.sh \
  | TCL_LSP_ASSUME_YES=1 sh
```

### Installer environment overrides

| Variable | Default | Effect |
|----------|---------|--------|
| `TCL_LSP_VERSION` | `latest` | Pin a release tag (e.g. `v1.2.3`). |
| `TCL_LSP_PREFIX`  | *prompt* (default `$HOME/.local/bin`) | Install directory. Set to bypass the interactive picker. |
| `TCL_LSP_ONLY`    | `both` | `tcl`, `f5`, or `both`. |
| `TCL_LSP_NO_DEPS` | unset | Skip Python install attempts (fail loudly instead). |
| `TCL_LSP_NO_PATH` | unset | Do not modify the shell rc file. |
| `TCL_LSP_NO_COMP` | unset | Skip shell completion install. |
| `TCL_LSP_ASSUME_YES` | unset | Answer "yes" to every prompt (required for unattended PATH / completion install). |
| `TCL_LSP_ASSUME_NO`  | unset | Answer "no" to every prompt (skip rc and completion entirely). |
| `TCL_LSP_REPO`    | `bitwisecook/tcl-lsp` | Source repository. |

Example — install only `f5`, system-wide, fully automated:

```sh
curl -fsSL https://raw.githubusercontent.com/bitwisecook/tcl-lsp/main/scripts/install.sh \
  | sudo TCL_LSP_ONLY=f5 TCL_LSP_PREFIX=/usr/local/bin TCL_LSP_ASSUME_YES=1 sh
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

## Updating

To pick up a new release, re-run the installer (it overwrites the
binaries in place):

```sh
curl -fsSL https://raw.githubusercontent.com/bitwisecook/tcl-lsp/main/scripts/install.sh | sh
```

Or, for a manual install, replace the file in `~/.local/bin` (or
`/usr/local/bin`) with the new release artefact.

---

## Uninstall

```sh
rm -f ~/.local/bin/tcl ~/.local/bin/f5
rm -f ~/.local/share/bash-completion/completions/{tcl,f5}
rm -f "${ZDOTDIR:-$HOME}/.zsh/completions/_tcl" "${ZDOTDIR:-$HOME}/.zsh/completions/_f5"
rm -f ~/.config/fish/completions/{tcl,f5}.fish
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
