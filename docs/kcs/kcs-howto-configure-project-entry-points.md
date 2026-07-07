# KCS: How do I stop W120 warnings in files loaded by an entry file?

> **Audience:** User
> **Type:** How-To

## Applies to

all-editors, tcl-lsp-cli, diagnostic, warning

## Question

My project has one "entry" file that runs the `package require`s and
then `source`s the rest of my files. The individual files use those
packages' commands without their own `package require`, so I get
[W120](codes/kcs-diagnostic-w120-missing-package-require.md) warnings
on every one. How do I tell the server that the entry file already
required them?

## Before you start

- The files that do the sourcing and the files being sourced are all
  inside the same workspace folder the editor opened.

## Answer

There are two ways. The first needs no configuration; use the second
only when the first cannot apply.

### Automatic — the workspace `source` graph

The server reads every `source FILE` statement and every
`package require NAME` across the workspace and builds a `source` graph
from them. A file inherits the `package require`s of **every** file
that `source`s it, directly or through a chain — so a
`package require Tk` in the entry file covers the module it sources, and
the module's W120 for a Tk command disappears on its own.

```tcl
# app.tcl  (the entry file)
package require Tk
source lib/widgets.tcl

# lib/widgets.tcl  (sourced module — no package require needed)
ttk::button .b -text OK      ;# no W120: Tk inherited from app.tcl
```

This works only for a **literal** path. A computed path the server
cannot resolve without running the code — `source [file join $dir x.tcl]`
or `source $module` — produces no graph edge, so the module does not
inherit anything. Use the manual list for that case.

### Manual — the `entryPoints` list

List your entry files in a project `.tcl-lsp.ini` at the workspace root,
one per line (paths relative to that folder, or absolute):

```ini
[project]
entryPoints =
    main.tcl
    src/app.tcl
```

The combined `package require`s of every listed file are then treated as
available across the **whole** project, whether or not the server can
trace a `source` to a given file. Setting `entryPoints` turns the
automatic `source`-graph inheritance **off** for that folder, so the
list is the single source of truth — include every entry your project
has.

Commit `.tcl-lsp.ini` with your source so the whole team shares it. See
[what config sections are valid](kcs-qa-what-config-sections-are-valid.md)
for the full INI schema.

## How to tell it worked

The W120 squiggles under the package's commands in the sourced files
disappear, while a genuinely missing `package require` — a command whose
package no entry file requires — still warns.

## Related

- [KCS index](README.md)
- [W120 — why does the analyser want a package require?](codes/kcs-diagnostic-w120-missing-package-require.md)
- [What sections and keys are valid in tcl-lsp config files?](kcs-qa-what-config-sections-are-valid.md)
- [How does tcl-lsp load configuration, and what overrides what?](kcs-qa-how-tcl-lsp-loads-configuration.md)
- [Glossary](../GLOSSARY.md)
