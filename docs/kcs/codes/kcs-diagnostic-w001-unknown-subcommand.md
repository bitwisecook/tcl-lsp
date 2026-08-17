# KCS: W001 — Why does the analyser flag an unknown subcommand?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, command-walk

## Profiles

default

## Question

Why do I see a warning on a subcommand that the analyser does not recognise?

## Why

An unrecognised subcommand usually means a typo or a version mismatch. At runtime, Tcl will raise an error because the ensemble or command does not support that subcommand.

W001 does not fire when the command name itself resolves to something other than the built-in ensemble — a same-file `proc`, an `oo::class`/snit/itcl class, an `interp alias`, or a `namespace ensemble create -command` all take priority over a built-in of the same name, exactly as Tcl resolves them at the call site. For example, a script that defines its own `proc string {op args} {...}` is never flagged for `string reverse $x`, because that call never reaches the built-in `string` ensemble at all.

W001 also fires on a **Tk/ttk widget's own instance command** — `.t bogus`, `$w bogus` — when the analyser can trace the receiver back to the widget that created it (`ttk::treeview .t`, or `set w [listbox .l]`). `configure` and `cget` are never flagged even though no widget spec lists them: every Tk widget accepts both universally.

## Symptoms

- A yellow squiggle appears under the subcommand token, with the message "unknown subcommand 'foo' for 'string'" (or "unknown subcommand 'foo' for widget 'listbox'").

## Example that triggers it

```tcl
string mach $a $b
```

```tcl
ttk::treeview .t
.t bogus
```

The analyser reports **`W001`** on the `mach` token in the first example, and on `bogus` in the second.

## Fix

```tcl
string match $a $b
```

```tcl
.t instate {selected} { puts "selected" }
```

Correct the subcommand name to one the command actually supports.

## How to suppress

Add `# noqa: W001` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [command walk](../../GLOSSARY.md#command-walk)
- Related codes: `E001`, `W002`
