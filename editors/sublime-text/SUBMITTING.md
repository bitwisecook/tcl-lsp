# Publishing LSP-Tcl

`LSP-Tcl` belongs in the
[SublimeLSP helper repository](https://github.com/sublimelsp/repository), not
directly in Package Control's default channel. Users still discover and
install it through Package Control.

## Distribution model

The extension source stays in `editors/sublime-text`. Tagged CI builds the
platform-independent `LSP-Tcl.sublime-package` release asset. The helper then
downloads the matching native server for the user's platform and verifies its
SHA-256 against `server_version.json` inside the package.

No mirror repository or per-release registry PR is required. The one-time
SublimeLSP entry follows stable GitHub release assets by name. Odd-minor
pre-releases publish `LSP-Tcl-prerelease.sublime-package`, which the entry does
not select.

## Repository entry

Add this object to the `packages` array in `repository.json`, in alphabetical
order:

```json
{
	"details": "https://github.com/bitwisecook/tcl-lsp",
	"issues": "https://github.com/bitwisecook/tcl-lsp/issues",
	"labels": [
		"lsp",
		"tcl"
	],
	"name": "LSP-Tcl",
	"readme": "https://raw.githubusercontent.com/bitwisecook/tcl-lsp/rust/editors/sublime-text/README.md",
	"releases": [
		{
			"asset": "LSP-Tcl.sublime-package",
			"python_versions": [
				"3.8"
			],
			"sublime_text": ">=4107"
		}
	]
}
```

The first release eligible for this entry is v2.2.2. v2.2.1 carries the
previous `TclLsp.sublime-package` shape and remains useful only as the tested
baseline.

## Pre-submission checks

1. Confirm the stable GitHub release carries `LSP-Tcl.sublime-package`.
2. Run `make build-editor-sublime`.
3. Run the Package Control reviewer against `build/sublime-stage` and fix all
   failures.
4. Install the built package together with `LSP` in Sublime Text and verify a
   Tcl file starts the server.
5. Open the one-time PR against `sublimelsp/repository`.

The package deliberately contains no key bindings, context menus, language
commands, custom syntax, completions, snippets, spec packs, or native
executable. Its only command-palette entry opens settings through Sublime's
built-in `edit_settings` command. Sublime's built-in `TCL` package owns syntax
and snippets; the standalone server embeds its default spec-pack data.
