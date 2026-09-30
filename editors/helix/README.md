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

## Configuration

tcl-lsp is not yet in
[`helix-editor/helix`](https://github.com/helix-editor/helix)'s default
`languages.toml`, so add the blocks below to your own.

```toml
# The released binary, or a local build from `make rust-server`.
[language-server.tcl-lsp]
command = "/path/to/tcl-lsp-server"
args = []

# <generated: helix-languages> one block per environment with a file type or shebang of its own
# Plain Tcl and Tk. Sends languageId "tcl": the server detects the dialect.
[[language]]
name = "tcl"
scope = "source.tcl"
file-types = ["tcl", "tk", "itcl", "tm", "test"]
shebangs = ["tclsh"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }

# Each environment that owns a file type or a shebang interpreter gets a
# language entry of its own, so Helix sends a distinct `language-id`: the id
# the server resolves to that environment.

# Expect
[[language]]
name = "expect"
language-id = "tcl-expect"
scope = "source.tcl"
file-types = ["exp", "expect"]
shebangs = ["expect"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }

# F5 BIG-IP
[[language]]
name = "f5-bigip"
language-id = "tcl-bigip"
scope = "source.tcl"
file-types = ["scf"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }

# F5 iApps
[[language]]
name = "f5-iapps"
language-id = "tcl-iapp"
scope = "source.tcl"
file-types = ["iapp", "iappimpl", "impl", "apl"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }

# F5 iRules
[[language]]
name = "f5-irules"
language-id = "tcl-irule"
scope = "source.tcl"
file-types = ["irul", "irule", "irules"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }

# F5 tmsh Scripts
[[language]]
name = "f5-tmsh"
language-id = "tcl-tmsh"
scope = "source.tcl"
file-types = ["tmsh"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }

# Jim Tcl
[[language]]
name = "jim"
language-id = "tcl-jim"
scope = "source.tcl"
shebangs = ["jimsh"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }

# SpecTcl
[[language]]
name = "spectcl"
language-id = "tclspec"
scope = "source.tcl"
file-types = ["tclspec"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }

# SslicTcl
[[language]]
name = "sslictcl"
language-id = "sslictcl"
scope = "source.tcl"
file-types = ["sslictcl"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }

# Tcl 8.4
[[language]]
name = "tcl8.4"
language-id = "tcl84"
scope = "source.tcl"
shebangs = ["tclsh8.4", "wish8.4"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }

# Tcl 8.5
[[language]]
name = "tcl8.5"
language-id = "tcl85"
scope = "source.tcl"
shebangs = ["tclsh8.5", "wish8.5"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }

# Tcl 8.6
[[language]]
name = "tcl8.6"
language-id = "tcl86"
scope = "source.tcl"
shebangs = ["tclsh8.6", "wish8.6"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }

# Tcl 9.0
[[language]]
name = "tcl9.0"
language-id = "tcl90"
scope = "source.tcl"
shebangs = ["tclsh9.0", "wish9.0"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }

# Tcl 9.1
[[language]]
name = "tcl9.1"
language-id = "tcl91"
scope = "source.tcl"
shebangs = ["tclsh9.1", "wish9.1"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }

# Cadence Genus / Innovus / Xcelium
[[language]]
name = "cadence-eda-tcl"
language-id = "tcl-cadence"
scope = "source.tcl"
file-types = ["globals"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }

# Intel Quartus Prime
[[language]]
name = "intel-quartus-eda-tcl"
language-id = "tcl-quartus"
scope = "source.tcl"
file-types = ["qsf", "qpf", "qip"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }

# Siemens Questa / ModelSim
[[language]]
name = "mentor-eda-tcl"
language-id = "tcl-mentor"
scope = "source.tcl"
file-types = ["do"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }

# Synopsys DC / PrimeTime / ICC2 / Formality
[[language]]
name = "synopsys-eda-tcl"
language-id = "tcl-synopsys"
scope = "source.tcl"
file-types = ["sdc", "upf"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }

# Tk
[[language]]
name = "tk"
language-id = "tk"
scope = "source.tcl"
shebangs = ["wish"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }

# Xilinx Vivado
[[language]]
name = "xilinx-eda-tcl"
language-id = "tcl-xilinx"
scope = "source.tcl"
file-types = ["xdc"]
comment-tokens = ["#"]
indent = { tab-width = 4, unit = "    " }
language-servers = ["tcl-lsp"]
auto-pairs = { "{" = "}", "[" = "]", "(" = ")", "\"" = "\"" }
# </generated>
```

The `[[language]]` blocks above are generated from the environment registry by
`cargo xtask gen-editor-configs`, and CI fails if they drift — so this block
stays in step with what the server actually routes. Do not hand-edit them. An
environment that owns an extension or a shebang interpreter gets a block of its
own; each block's `language-id` is the language id VS Code sends for the same
environment.

Helix keys `file-types` on extension only, so a vendor script saved as plain
`.tcl` still needs a `# tcl-dialect:` comment or the `dialect` setting below,
and the BIG-IP config *basenames* (`bigip.conf`, `bigip_base.conf`, …) are
reached by the server's own basename routing once the file is open.

<!-- <generated: helix-unlisted> -->
`bpf` and `microchip-libero-eda-tcl` have no file extension or shebang word of
their own, so they have no entry. Select them per file with a `# tcl-dialect:`
comment or per workspace with the `dialect` setting below.
<!-- </generated> -->

## Settings

Pass workspace settings via the `config` key:

```toml
[language-server.tcl-lsp.config.tclLsp]
# <generated: helix-dialects>
# Valid dialects: bpf, expect, f5-bigip, f5-iapps, f5-irules, f5-tmsh, jim,
# spectcl, sslictcl, tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, cadence-eda-tcl,
# intel-quartus-eda-tcl, mentor-eda-tcl, microchip-libero-eda-tcl,
# synopsys-eda-tcl, tk, xilinx-eda-tcl
# </generated>
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
