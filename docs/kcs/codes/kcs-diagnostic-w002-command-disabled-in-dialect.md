# KCS: W002 — Why is a command disabled in the active dialect?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn that a command is disabled in the current dialect?

## Why

Some Tcl dialects (e.g. iRules) deliberately restrict the set of available commands. Using a disabled command will fail at runtime in that environment, even though it works in standard Tcl.

## Symptoms

- A yellow squiggle appears under the command name (or, for a version-gated
  subcommand such as `package files`, under `command subcommand` together),
  with a message like "'exec' is disabled in the active dialect profile
  (available in: tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, f5-iapps, …)". The
  "available in" list is read straight from the command's registry entry, so
  it names every dialect the command actually works in — a quick way to tell
  whether switching the file's dialect would fix it.

## Example that triggers it

```tcl
# tcl-dialect: f5-irules
exec ls /tmp
```

The analyser reports **`W002`** on the `exec` token.

## Fix

```tcl
# Remove or replace with a dialect-appropriate alternative.
log local0. "listing not available in iRules"
```

Use only commands permitted by the active dialect.

W002 only fires for a **literal** command name that the registry can
statically resolve as *known somewhere, disabled here* — a `$var`/`[cmd]`
command head is a runtime dispatch (see W307's KCS note instead), and a call
that resolves to a same-file `proc`, `interp alias`, static `rename` target,
`TclOO`/snit/itcl class, `namespace ensemble create` namespace, or `#
tcl-lsp: stub` declaration is never flagged — Tcl resolves the call to that
definition, not to the disabled builtin, so there is nothing to warn about.

## How to suppress

Add `# noqa: W002` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W001`, `E001`
