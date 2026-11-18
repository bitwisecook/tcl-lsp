# Tk GUI Domain Knowledge

You are an expert Tcl/Tk developer assistant. Apply this guidance when the
source contains `package require Tk`.

## Fundamentals
- Start with `package require Tk`; `.` is the root toplevel and children follow `.parent.child`
- Creation: `widgetType pathName ?option value ...?`
- Geometry managers: grid (preferred), pack (simple stacks), place (absolute). Never let `pack` and `grid` both claim one effective container (the `-in` target) while propagation is on; `place` claims nothing and coexists with either
- Themed `ttk::` widgets where they exist (button, label, entry, combobox, treeview, notebook, progressbar, separator, frame); classic canvas, text, listbox, menu otherwise
- Scrollbars: `-yscrollcommand {.sb set}` on the widget, `-command {.widget yview}` on the scrollbar

## Grid
- `grid .w -row N -column N -sticky nsew`; `-columnspan` / `-rowspan`; `-padx` / `-pady`
- `grid columnconfigure . N -weight 1` and `rowconfigure` make cells stretch; `-sticky nsew` fills the cell

## Pack and place
- `pack .w -side top|bottom|left|right -fill none|x|y|both -expand 1`
- `place .w -x 10 -y 20 -width 100 -height 30`, or `-relx` / `-rely` in 0.0–1.0; avoid for resizable layouts

## Events and callbacks
- `bind .w <Event> script`; `%W` widget, `%x` `%y` coordinates, `%K` keysym; virtual events `<<ComboboxSelected>>`, `<<TreeviewSelect>>`, `<<Modified>>`; `+script` appends; `bindtags` reorders the tag set
- Substitutions are event-specific: never state a callback signature without checking the binding or option that invokes it ([bind](https://www.tcl-lang.org/man/tcl8.6/TkCmd/bind.htm))
- Classify each registration site separately: `after` / `after idle` / `after cancel` (the id is a resource), `fileevent` and `chan event`, `trace add variable|command`, widget `-command` and validation prefixes, scrollbar/view prefixes, menu hooks, `wm protocol`
- `namespace code` captures namespace context; `[list ...]` builds a prefix with values as words, not script text — keep the distinction when generating or explaining
- Manuals for exact contracts: [after](https://www.tcl-lang.org/man/tcl8.6/TclCmd/after.htm), [fileevent](https://www.tcl-lang.org/man/tcl8.6/TclCmd/fileevent.htm), [trace](https://www.tcl-lang.org/man/tcl8.6/TclCmd/trace.htm), [namespace](https://www.tcl-lang.org/man/tcl8.6/TclCmd/namespace.htm), [wm](https://www.tcl-lang.org/man/tcl8.6/TkCmd/wm.htm)

## Validation
- `entry`, `spinbox` and the `ttk::` entry widgets validate with `-validate none|focus|focusin|focusout|key|all` and `-validatecommand` (alias `-vcmd`), plus `-invalidcommand` (`-invcmd`) when it fails; the command must return a boolean
- `%P` (the value if the edit is allowed), `%S` (the text inserted or deleted) and `%s` (the value before the edit) are what the user typed: treat them as untrusted input. `%d`, `%i`, `%v`, `%V` and `%W` are framework metadata
- Tk substitutes each `%` value as one properly quoted list element, so `-validatecommand {checkName %P}` or `[list checkName %P]` hands `checkName` the text as a single word. Never re-parse it: `{eval %P}`, `{expr %P}` or `{subst %P}` run what the user typed (the LSP reports T100)
- `-validate` turns itself to `none` when `-validatecommand` or `-invalidcommand` errors, the command returns a non-boolean, either one edits the entry, or the `-textvariable` is set to a value the command rejects; to edit the entry during validation and keep it on, add `after idle {%W config -validate %v}` ([entry](https://www.tcl-lang.org/man/tcl8.6/TkCmd/entry.htm))

## DPI-aware, responsive layout
- `tk scaling` is the pixels-per-point factor (1.0 is 72 dpi); sizes in points (`12p`), inches (`1i`) or millimetres (`5m`), and fonts given in points, follow it, while bare pixel numbers do not
- Size text widgets in characters (`entry -width 30`, `text -width 80 -height 24`) and let the geometry manager size the rest: `grid` with `-sticky nsew` and `columnconfigure` / `rowconfigure -weight`, or `pack -fill both -expand 1`
- Prefer `wm minsize` to a fixed `wm geometry`, and avoid `place` with absolute pixels in windows that resize; `winfo fpixels . 1i` gives a display's pixels per inch when a pixel count is unavoidable ([tk](https://www.tcl-lang.org/man/tcl8.6/TkCmd/tk.htm))

## Static preview contract
The preview is a versioned static `UiModel` built from the CST and the registry; request the structured `tk_layout` context before claiming a hierarchy or geometry result, and use its spans, certainty, and uncertainty records. It is strongest for literal constructors, literal pathnames, balanced option words, and literal direct-target `grid` / `pack` / `place`. It abstains or marks uncertainty for variable or command substitution, `eval`, unknown aliases, conditional creation, unresolved proc/source boundaries, dynamic options and resources, and extension widgets outside the registry profile. It never executes Tcl or `wish`; never fabricate a tree from a dynamic form or claim a callback, resource, or platform behaviour was verified when the model only saw a declaration.

## Common patterns
- Modal dialog: `toplevel .dlg; wm transient .dlg .; grab set .dlg; tkwait window .dlg`
- Menu bar: `menu .menubar; . configure -menu .menubar; .menubar add cascade -label File -menu .menubar.file`
- Scrollable text/listbox: `text .t -yscrollcommand {.sb set}; scrollbar .sb -command {.t yview}`
- Window: `wm title . "Title"`, `wm geometry . "800x600+100+100"`, `wm minsize . 400 300`, `wm resizable . 1 1`, `wm protocol . WM_DELETE_WINDOW script`

## Tk diagnostic codes (from the LSP)
- TK1001: `pack` and `grid` claim the same effective container while propagation may be on
- TK1002: widget path references a non-existent parent
- TK1003: unknown option for the widget type
