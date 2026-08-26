# KCS: How do I bind keys to the Tcl commands in Sublime Text?

> **Audience:** User
> **Type:** How-To

## Applies to

sublime-text

## Question

How do I add keyboard shortcuts for the TclLsp package's commands (Format
Document, Minify, Select Dialect, Restart Language Server, and the rest)
in Sublime Text?

## Before you start

- Install the **TclLsp** package (see the package
  [README](../../editors/sublime-text/README.md)).
- The package ships **no** key bindings and no example keymap, so nothing
  it adds can clash with shortcuts you already use — every binding is
  yours to choose. This is what Package Control asks of a package, and it
  keeps a fresh install from quietly taking chords you rely on.

## Answer

Open **Preferences > Key Bindings**. Sublime opens two panes: the default
bindings on the left (read-only) and your own user keymap on the right.
Add entries to the right-hand pane.

Each entry binds one command. The commands are the same ones listed under
**Tcl:** in the Command Palette:

| Command | Binding role |
|---|---|
| `lsp_format_document` | Format the current document |
| `tcl_fix_all_safe_issues` | Apply all safe quick fixes |
| `tcl_optimise_document` | Apply all optimisation suggestions |
| `tcl_minify_document` | Minify the current document |
| `tcl_unminify_error` | Translate a minified error message back to original names |
| `tcl_select_dialect` | Choose the active Tcl dialect |
| `tcl_restart_server` | Restart the Tcl language server |
| `tcl_recommended_setup` | Re-offer the package's recommended settings |

A user keymap entry looks like this (Windows/Linux shown; on macOS use
`super` in place of `ctrl`):

```json
[
	{"keys": ["ctrl+alt+f"], "command": "lsp_format_document"},
	{"keys": ["ctrl+alt+q"], "command": "tcl_fix_all_safe_issues"},
	{"keys": ["ctrl+alt+d"], "command": "tcl_select_dialect"}
]
```

Pick chords that don't collide with bindings you already use — the
left-hand pane is the full list of what Sublime Text already claims.

The format, fix, optimise, and minify commands act on the current
document, so they are most useful while a Tcl or iRules file has focus.
To keep a chord free everywhere else, add a scope `context` to the
binding:

```json
{
	"keys": ["ctrl+alt+f"],
	"command": "lsp_format_document",
	"context": [
		{"key": "selector", "operand": "source.tcl, source.irule"}
	]
}
```

## How to tell it worked

Save the user keymap (the right-hand pane), then press the chord you
bound while a Tcl file is open. The command runs immediately — for
example, `lsp_format_document` reformats the buffer. If nothing happens,
open the console (**View > Show Console**) and press the chord again:
Sublime logs the command it dispatched, which shows whether another
binding claimed the chord first.

## Related

- [KCS index](README.md)
- [Sublime Text package README](../../editors/sublime-text/README.md)
- [kcs-qa-when-to-restart-server.md](kcs-qa-when-to-restart-server.md)
- [Glossary](../GLOSSARY.md)
