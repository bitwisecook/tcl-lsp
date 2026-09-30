# Tcl LSP for Zed

Language-server support for Tcl, iRules, iApps, APL, TMSH, and Expect,
powered by [tcl-lsp](https://github.com/bitwisecook/tcl-lsp).

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

The `languages/*/config.toml` suffix lists are generated from tcl-lsp's
environment registry by `cargo xtask gen-editor-extensions`, and the language
table in `extension.toml` by `cargo xtask gen-editor-configs`; drift gates in
`make xtask-check` fail when an environment or suffix is added without
regenerating. The APL and BIG-IP tree-sitter grammars live in this repository
under `grammars/` and are fetched by commit — bump the `rev` in
`extension.toml` whenever either changes.

The server detects the dialect from the file name and content. Jim Tcl has no
Zed language of its own: a Jim script opens as Tcl, and the server's shebang
tier selects `jim` from a `#!/usr/bin/env jimsh` line. A Jim script without
that line takes a `# tcl-dialect: jim` comment or the `dialect` setting. To
force a dialect, add a Zed setting such as:

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

<!-- <generated: zed-dialects> -->
`tclLsp.dialect` takes any of `bpf`, `expect`, `f5-bigip`, `f5-iapps`,
`f5-irules`, `f5-tmsh`, `jim`, `spectcl`, `sslictcl`, `tcl8.4`, `tcl8.5`,
`tcl8.6`, `tcl9.0`, `tcl9.1`, `cadence-eda-tcl`, `intel-quartus-eda-tcl`,
`mentor-eda-tcl`, `microchip-libero-eda-tcl`, `synopsys-eda-tcl`, `tk` and
`xilinx-eda-tcl`.
<!-- </generated> -->

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
