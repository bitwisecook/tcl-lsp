# KCS: W004 — Command option is not available in the active dialect

> **Audience:** User
> **Type:** Issue

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

Replace the command call with an equivalent that is valid in the active dialect, or change the dialect setting to a version that supports the option.

## How to suppress

Add `# noqa: W004` at the end of the offending line.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [lowering](../../GLOSSARY.md#lowering)
- Related codes: `W002`, `W003`
