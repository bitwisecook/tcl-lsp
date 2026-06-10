# KCS: How do I bind keys to the Tcl commands in Sublime Text?

> **Audience:** User
> **Type:** How-To

## Applies to

sublime-text

## Question

How do I add keyboard shortcuts for the Tcl package's commands (Format
Document, Minify, Select Dialect, Restart Language Server, and the rest)
in Sublime Text?

## Before you start

- Install the **Tcl** package (see the package
  [README](../../editors/sublime-text/README.md)).
- The package ships **no** key bindings by default, so nothing it adds
  can clash with shortcuts you already use.

## Answer

Open **Preferences > Package Settings > Tcl > Key Bindings**. Sublime
opens two panes: the package's example keymap on the left (read-only)
and your own user keymap on the right. Copy any binding you want from
the left pane into the right pane and uncomment it.

The example keymap is platform-aware — the menu opens the
`Example (Windows)`, `Example (Linux)`, or `Example (OSX)` keymap to
match the machine you are on, and saves your choices into your user
`Default (<platform>).sublime-keymap`.

Each entry binds one command. The commands are the same ones listed
under **Tcl:** in the Command Palette:

| Command | Binding role |
|---|---|
| `lsp_format_document` | Format the current document |
| `tcl_fix_all_safe_issues` | Apply all safe quick fixes |
| `tcl_optimise_document` | Apply all optimisation suggestions |
| `tcl_minify_document` | Minify the current document |
| `tcl_unminify_error` | Translate a minified error message back to original names |
| `tcl_select_dialect` | Choose the active Tcl dialect |
| `tcl_restart_server` | Restart the Tcl language server |

A user keymap entry looks like this (Windows/Linux shown; on macOS use
`super` in place of `ctrl`):

```json
[
	{"keys": ["ctrl+alt+f"], "command": "lsp_format_document"},
	{"keys": ["ctrl+alt+q"], "command": "tcl_fix_all_safe_issues"},
	{"keys": ["ctrl+alt+d"], "command": "tcl_select_dialect"}
]
```

Pick chords that don't collide with bindings you already use; the
example keymap leaves every suggestion commented out for exactly this
reason. The format, fix, optimise, and minify commands act on the
current document, so they are most useful bound while a Tcl or iRules
file has focus.

## How to tell it worked

Save the user keymap (the right-hand pane), then press the chord you
bound while a Tcl file is open. The command runs immediately — for
example, `lsp_format_document` reformats the buffer. You can also
confirm the binding is live in **Preferences > Key Bindings** or via the
command's entry in the Command Palette.

## Related

- [KCS index](README.md)
- [Sublime Text package README](../../editors/sublime-text/README.md)
- [kcs-qa-when-to-restart-server.md](kcs-qa-when-to-restart-server.md)
- [Glossary](../GLOSSARY.md)
