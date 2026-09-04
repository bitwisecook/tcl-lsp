# LSP-Tcl

Tcl language-server support for Sublime Text, powered by
[tcl-lsp](https://github.com/bitwisecook/tcl-lsp).

LSP-Tcl is intentionally a small helper package. Sublime Text's built-in
`TCL` package remains the owner of Tcl syntax highlighting and snippets;
LSP-Tcl only registers the language server and fills in file associations for
Tcl-family suffixes that Sublime would otherwise open as plain text.

## Installation

Install both packages with Package Control:

1. **LSP**
2. **LSP-Tcl**

The first time a Tcl file starts the server, LSP-Tcl downloads the native
`tcl-lsp-server` for the current platform from the matching tcl-lsp GitHub
release. The download must match the SHA-256 digest pinned into the released
package. macOS, Linux, and Windows on x64 and arm64 are supported.

To use a server you installed yourself, open **Preferences > Package Settings
> LSP > Servers > LSP-Tcl** and set:

```json
{
    "server_path": "/path/to/tcl-lsp-server"
}
```

## Features

The server provides diagnostics, context-aware completion, hover help, go to
definition and references, document symbols, formatting, rename, code actions,
semantic highlighting, signature help, folding, inlay hints, and call
hierarchy.

The default dialect is Tcl 8.6. Change `settings.tclLsp.dialect` in LSP-Tcl's
settings for Tcl 8.4–9.1, F5 iRules/iApps/tmsh/BIG-IP, Expect, SpecTcl,
SslicTcl, BPF, or supported EDA Tcl dialects.

LSP-Tcl deliberately ships no key bindings, context menu, custom syntax,
completions, or snippets. Use the base LSP package's command palette commands
and Sublime Text's built-in Tcl resources.

## Development install

Symlink this directory into Sublime Text's Packages directory as `LSP-Tcl`,
then install the base `LSP` package. A locally built server may be placed at
`server/tcl-lsp-server` inside the symlinked directory; it takes precedence
over downloads.

See the repository's
[editor installation guide](../../INSTALL-editors.md#sublime-text) for manual
installation and troubleshooting.
