# KCS: W211 — Why does the analyser warn about a variable that is set but never read?

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors, diagnostic, liveness

## Profiles

default

## Question

Why does the analyser flag a variable that is assigned a value but never used?

## Why

Unused variables waste memory and make the code harder to read; they often indicate a logic error where a result was computed but forgotten.

## Symptoms

- A faint hint-severity underline (the subtle "three dots") appears under the variable name, with the message "variable set but never used". Raise its prominence with `tclLsp.diagnosticSeverity.W211` (see [How to suppress](#how-to-suppress)).

## Example that triggers it

```tcl
set result [expr {1 + 1}]
```

The analyser reports **`W211`** because `result` is never read.

## Fix

Remove the unused assignment, or use the variable:

```tcl
set result [expr {1 + 1}]
puts $result
```

## How to suppress

Add `# noqa: W211` at the end of the offending line, or set
`tclLsp.diagnostics.W211` to `false` to turn the check off entirely.

## How to change its severity

If the default hint is too subtle (or too loud), re-level it without disabling
it. In VS Code settings:

```json
{ "tclLsp.diagnosticSeverity.W211": "warning" }
```

Accepted values are `"error"`, `"warning"`, `"information"`, and `"hint"` (the
default). Any diagnostic code can be re-levelled with
`tclLsp.diagnosticSeverity.<CODE>`; this changes only how the editor renders the
diagnostic, never the analysis.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [liveness](../../GLOSSARY.md#liveness)
- Related codes: `W210`, `W214`, `W220`
