# KCS: How do I bind keys to the Tcl commands in Sublime Text?

> **Audience:** User
> **Type:** How-To

## Applies to

sublime-text

## Question

How do I add a keyboard shortcut for an LSP command such as Format Document in
Sublime Text?

## Before you start

- Install **LSP** and **LSP-Tcl** (see the helper
  [README](../../editors/sublime-text/README.md)).
- LSP-Tcl ships no commands or key bindings. Use the base LSP package's
  commands and choose any personal bindings in your user keymap.

## Answer

Open **Preferences > Key Bindings**. Sublime opens two panes: the default
bindings on the left (read-only) and your own user keymap on the right.
Add entries to the right-hand pane.

A user keymap entry for the base LSP format command looks like this
(Windows/Linux shown; on macOS use `super` in place of `ctrl`):

```json
[
	{"keys": ["ctrl+alt+f"], "command": "lsp_format_document"}
]
```

Pick chords that don't collide with bindings you already use — the
left-hand pane is the full list of what Sublime Text already claims.

To keep the chord free outside Tcl files, add a scope `context`:

```json
{
	"keys": ["ctrl+alt+f"],
	"command": "lsp_format_document",
	"context": [
		{"key": "selector", "operand": "source.tcl"}
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
