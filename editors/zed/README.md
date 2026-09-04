# Tcl LSP for Zed

Language-server support for Tcl, iRules, iApps, Expect, TMSH, and supported
EDA Tcl dialects, powered by
[tcl-lsp](https://github.com/bitwisecook/tcl-lsp).

## Features

The extension provides syntax highlighting and the tcl-lsp language server:
diagnostics, completion, hover help, navigation, symbols, formatting, rename,
code actions, semantic highlighting, signature help, folding, inlay hints,
call hierarchy, and type hierarchy.

It deliberately contains no snippets, slash commands, MCP server, or native
binary. On first use it downloads the `tcl-lsp-server` binary for the current
platform from the matching tcl-lsp release. Release builds pin that download
to the version in `extension.toml`.

## Installation

Search for **Tcl LSP** in Zed's extension panel and install it.

For a development install, run **Extensions: Install Dev Extension** from
Zed's command palette and select this directory.

## File and dialect tracking

The `languages/*/config.toml` suffix lists are generated from tcl-lsp's Rust
dialect catalog by `cargo xtask gen-editor-extensions`. The repository drift
gate prevents the extension from falling behind when a dialect or suffix is
added.

The server detects the dialect from the file name and content. To force one,
add a Zed setting such as:

```json
{
  "lsp": {
    "tcl-lsp": {
      "settings": {
        "tclLsp": {
          "dialect": "tcl8.6"
        }
      }
    }
  }
}
```

Supported profiles include Tcl 8.4–9.1, F5 iRules/iApps/tmsh/BIG-IP,
Expect, SpecTcl, SslicTcl, BPF, and the EDA Tcl dialects represented in the
catalog.

## Platforms

Prebuilt servers are published for macOS, Linux, and Windows on x64 and arm64.
A `tcl-lsp-server` already on `PATH` takes precedence; otherwise the extension
downloads the release selected by its `extension.toml` version.

## Publishing

`scripts/release/publish_zed.sh` prepares the one-time registration or version
bump in `zed-industries/extensions`. The central entry uses the `tcl-lsp`
submodule and `path = "editors/zed"` so Zed builds this directory directly.

## License

The Zed extension code in this directory is licensed under the
[GNU General Public License v3.0 or later](LICENSE). The tcl-lsp server it
downloads is a separate program and remains licensed under the repository's
GNU Affero General Public License v3.0 or later.
