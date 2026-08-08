# Helix

Helix has built-in LSP support. Add the following to your `languages.toml`
(typically `~/.config/helix/languages.toml`).

## Prerequisites

The `tcl-lsp-server` binary. It is self-contained — no Python, runtime, or
interpreter. Download the asset for your platform from
[Releases](https://github.com/bitwisecook/tcl-lsp/releases/latest) or build
it with `make rust-server`.

See [The server binary](../../INSTALL-editors.md#the-server-binary) in the
installation guide for the per-platform asset names.

## Upstream integration (after merge)

Once tcl-lsp is added to
[`helix-editor/helix`](https://github.com/helix-editor/helix)'s default
`languages.toml`, Helix users only need `tcl-lsp-server` on their PATH — no
per-user `languages.toml` edit is required.

## Configuration (until upstream merges)

```toml
# The released binary, or a local build from `make rust-server`.
[language-server.tcl-lsp]
command = "/path/to/tcl-lsp-server"
args = []

# Core Tcl / Tk. Sends languageId "tcl" → the server's default dialect.
[[language]]
name = "tcl"
scope = "source.tcl"
file-types = ["tcl", "tk", "itcl", "tm"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }

# The dialect-specific file types need their OWN language entry so Helix sends a
# distinct `language-id` — routing every extension through `name = "tcl"` sends
# languageId "tcl", which the server maps to tcl8.6, so F5 iRules / iApps and
# Expect analysis never engages. `language-id` sets the LSP id the server keys
# its dialect on (see `dialect_from_language_id`).
[[language]]
name = "f5-irules"
language-id = "f5-irules"
scope = "source.tcl"
file-types = ["irul", "irule"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }

[[language]]
name = "f5-iapps"
language-id = "f5-iapps"
scope = "source.tcl"
file-types = ["iapp", "iappimpl", "impl", "apl"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }

[[language]]
name = "expect"
language-id = "expect"
scope = "source.tcl"
file-types = ["exp"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }
```

## Settings

Pass workspace settings via the `config` key:

```toml
[language-server.tcl-lsp.config.tclLsp]
# Valid dialects: tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, f5-irules, f5-iapps,
# f5-bigip, f5-tmsh, synopsys-eda-tcl, cadence-eda-tcl, xilinx-eda-tcl,
# intel-quartus-eda-tcl, mentor-eda-tcl, expect
dialect = "tcl8.6"

[language-server.tcl-lsp.config.tclLsp.formatting]
indentSize = 4
maxLineLength = 120
```

## Configuration File

tcl-lsp reads a platform-native configuration file for editor-agnostic
defaults (diagnostics, optimiser, shimmer, features, formatting):

| Platform | Default path |
|----------|-------------|
| Linux / BSD / WSL2 | `~/.config/tcl-lsp/config.ini` |
| macOS | `~/Library/Application Support/tcl-lsp/config.ini` |
| Windows | `%APPDATA%\tcl-lsp\config.ini` |
| MSYS2 / Cygwin | `~/.config/tcl-lsp/config.ini` |

`$XDG_CONFIG_HOME` overrides the default on every platform.

Settings from the config file are applied as baseline defaults.  Helix
`config` settings in `languages.toml` override the config file — so you
can set shared defaults in the config file and per-project overrides in
Helix.

See [docs/design/contracts/xdg-config.md](../../docs/design/contracts/xdg-config.md) for
the full reference.
