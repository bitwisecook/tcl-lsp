# KCS: W120 — Why does the analyser want a package require?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, lowering

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

## How to suppress

Add `# noqa: W120` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W001`, `W002`
