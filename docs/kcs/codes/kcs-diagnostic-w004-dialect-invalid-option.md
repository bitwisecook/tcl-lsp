# KCS: W004 — Command option is not available in the active dialect

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, lowering

## Profiles

default

## Question

Why does the analyser warn that a command option is not available in my dialect?

## Why

Some command options were added in later Tcl versions. When the active dialect is set to an older version, using an option that did not exist at that version will raise an error in the real Tcl interpreter. For example, `lsort -stride` was added in Tcl 8.6; `regsub -command`, `vwait -all`, and `socket -reuseaddr` require Tcl 9.0.

## Symptoms

- A yellow squiggle appears under the option token, with a message such as: "Option '-stride' on 'lsort' is not available in the active dialect (tcl8.4)."

## Example that triggers it

```tcl
# dialect: tcl8.4
lsort -stride 2 $l
```

The analyser reports **`W004`** on the `-stride` token.

## Fix

```tcl
# Remove the dialect-restricted option or raise the dialect setting:
lsort $l
```

Replace the command call with an equivalent that is valid in the active dialect, or change the dialect setting to a version that supports the option. A quick fix titled "Remove '-option'" is offered wherever the flag appears — it deletes the flag and its value, matching the fix above.

The check also resolves a unique-prefix subcommand abbreviation the same way real Tcl ensemble dispatch does, so `chan conf -inputmode raw` (`conf` ⇒ `configure`) is flagged exactly like the spelled-out form. It does not fire when the command name resolves to a same-file proc, `TclOO` class, `interp alias`, or ensemble instead of the registry built-in — the call really dispatches to that definition, so the built-in's dialect restriction no longer applies (e.g. a proc named `lsearch` that accepts its own `-stride` argument).

## How to suppress

Add `# noqa: W004` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W002`, `W003`
