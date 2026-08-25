# KCS: feature — Tk Preview

> **Audience:** User
> **Type:** Functionality

## Summary

Static preview pane for Tk GUI source that updates as you edit.

## Applies to

VS Code, JetBrains, Copilot Chat, MCP

## Availability

| Context | How |
|---------|-----|
| VS Code command | `Tcl: Open Tk Preview` |
| VS Code chat | `@tk /preview` |
| JetBrains action | `Show Tk UI Model` (validated JSON model) |
| MCP | `tk_layout` tool (the same structured static model) |

## How to use

- **VS Code**: Open a Tcl file and run `Tcl: Open Tk Preview`. The preview
  analyses the server's current document snapshot and updates as you edit.
  It may show a partial tree or uncertainty when the layout is dynamic.
- **VS Code chat**: `@tk /preview` opens the preview for the current file.
- **JetBrains**: Run **Show Tk UI Model** to open the validated static model as
  JSON. JetBrains does not yet render the visual approximation.
- **MCP**: `tk_layout` requests the same schema-versioned model for analysis or
  UI tooling (MCP source is supplied in the request, not an open-document
  snapshot).

## Operational context

The server builds a versioned `TkUiModel` from the Tcl CST and registry. The
model contains widget hierarchy, literal options, geometry evidence, source
spans, certainty, and explicit uncertainties. VS Code renders that model in
the Tk Preview pane; JetBrains currently presents the validated model JSON;
MCP exposes it to tools and agents. Clients reject a response if
its URI, document version, or schema version no longer matches the active
request, so an older analysis cannot overwrite a newer edit.

This is static analysis. It does not execute Tcl, `wish`, Tk callbacks,
`source`, packages, or workspace code. It is not pixel-perfect and does not
promise native theme, font, accessibility, or window-manager behavior.

## Failure modes

- Partial or unavailable preview when widget commands, pathnames, option names,
  or values are computed through variables, command substitutions, `eval`,
  unknown aliases, conditionals, or extension packages not present in the
  selected registry profile.
- Layout uncertainty when widgets are destroyed/recreated or creation occurs
  across an unresolved procedure/source boundary.
- Static geometry is evidence from the source, not a runtime measurement.
  `pack` and `grid` are exclusive claimants of their effective `-in`
  container; `place` does not claim or resize that container.
- Callback, resource, event-loop, theme, and platform behavior may be marked
  as planned/uncertain rather than rendered as if verified.

## Example

With this file open in the editor:

```tcl
package require Tk
frame .main
label .main.title -text "Welcome"
entry .main.name
button .main.ok -text "OK"
pack .main.title .main.name .main.ok -side top
pack .main
```

Running **Tcl: Open Tk Preview** opens a side panel rendering a
frame with a "Welcome" label, an empty entry box, and an "OK"
button stacked vertically. Edits to the file refresh the model — for
example, changing `-text "Welcome"` to `-text "Hello"` updates the rendered
label without running the program.

The static model is most reliable for literal constructors and literal
`grid`, `pack`, or `place` calls. A dynamic form is shown with its uncertainty
rather than silently substituted with made-up widgets.

## Further reading

- [Static Tk UI model](../../design/tk-static-ui-model.md)
- Official [Tk command index](https://www.tcl-lang.org/man/tcl8.6/TkCmd/contents.htm)
- Official [`bind` manual](https://www.tcl-lang.org/man/tcl8.6/TkCmd/bind.htm)
- Official [`wm` manual](https://www.tcl-lang.org/man/tcl8.6/TkCmd/wm.htm)

## Discoverability

- [KCS feature index](README.md)
- [VS Code extension contracts](../../../docs/design/contracts/vscode-extension.md)
