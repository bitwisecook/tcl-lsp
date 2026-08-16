# KCS: W120 — Why does the analyser want a package require?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why does the analyser warn that a command is used without a corresponding `package require`?

## Why

Without an explicit `package require`, the command may not be available at runtime. Adding the require makes the dependency visible, ensures the package is loaded, and helps other tools track dependencies.

## Symptoms

- A yellow squiggle appears under the command, with the message "command 'http::geturl' used without 'package require http'".

## Example that triggers it

```tcl
set tok [http::geturl "https://example.com"]
```

The analyser reports **`W120`** on the `http::geturl` call.

## Fix

```tcl
package require http
set tok [http::geturl "https://example.com"]
```

Add the appropriate `package require` before first use of the command.

## Multi-file projects — inheriting requires from an entry file

A project often has one "entry" file that runs the `package require`s
and then `source`s its individual modules. Those modules use the
required commands without a `package require` of their own. The server
does **not** flag them, for two reasons:

1. **Automatic (no configuration).** The server builds a workspace
   `source` graph from the `source FILE` statements it finds. A module
   inherits the `package require`s of every file that (transitively)
   `source`s it, so a `package require Tk` in the entry file covers the
   module it sources. Only literal `source path.tcl` targets are
   followed — a computed `source $dir/x.tcl` cannot be resolved
   statically.

2. **Explicit entry points.** When the automatic path can't help — for
   example the entry file uses a computed `source` path — list the
   entry files in `.tcl-lsp.ini`:

   ```ini
   [project]
   entryPoints =
       main.tcl
       src/app.tcl
   ```

   Their combined `package require`s are then treated as available
   across the whole project, and the automatic `source`-graph
   inheritance is turned off. See
   [what config sections are valid](../kcs-qa-what-config-sections-are-valid.md).

## Packages that are only a binary extension

Some packages ship no Tcl at all — their `pkgIndex.tcl` just `load`s a
shared object, and the directory holds nothing else:

```tcl
package ifneeded pix 0.8 [list apply {dir { load [file join $dir libpix.so] Pix }} $dir]
```

The server treats such a package as **known but opaque**: it exists, so
requiring it is fine and nothing complains about the package itself, but
which commands it installs cannot be worked out without running it, so no
claim is made about them either way.

The practical consequence is that requiring one of these no longer silences
W120 for *other* packages in the same file. Before, a file containing
`package require pix` lost every W120 it had, including one about a
completely unrelated missing `package require http`.

## A W120 that clears itself a moment after a workspace opens

If an editor restores several tabs on startup, a module that inherits its
`package require` from an entry file (see above) can briefly show a
false-positive W120 until the server finishes scanning the workspace for
`source` ancestors and package providers. The server re-checks every open
document's diagnostics once that scan completes, so the warning should
disappear on its own within the same startup window — no edit or manual
restart needed. If a false-positive W120 persists after the workspace has
clearly finished loading (the status bar shows the server is idle), that is
a bug — open an issue with the workspace layout.

## The quick fix, and when it is offered

Two things can offer to add a `package require`, and both need evidence.

**W120's own fix** is precise: the diagnostic already knows, from the command
registry, exactly which package the command belongs to. Applying it inserts
the require line at the top of the file.

**The suggestion for a command the registry does not know** is the fallback,
and it is offered only when *all* of these hold:

- the cursor is on a **command head** — an actual call, not a mention of the
  same identifier in a comment, a quoted or braced string, an argument word,
  or the name word of a `proc` definition;
- the head is written out literally, not computed (`$cmd`, `[pick]`);
- the head's leading namespace **exactly** names a package the registry
  knows — `json::write` is evidence for `json`; `jsonify` is evidence for
  nothing;
- nothing else in the workspace answers to that name (no registry command, no
  same-file or imported definition, no static `rename` or `interp alias`);
- the file contains no computed `package require` or `load`, which could
  register the command at run time;
- the package is not already required.

Applying either fix **loads the package and runs its initialisation code**,
so neither is applied by
[Fix All Safe Issues](../kcs-qa-what-does-fix-all-safe-issues-apply.md) —
both are classified behaviour-hardening, and you accept them one at a time.

## How to suppress

Add `# noqa: W120` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `W001`, `W002`
