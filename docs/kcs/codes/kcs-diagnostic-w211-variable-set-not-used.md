# KCS: W211 — Why does the analyser warn about a variable that is set but never read?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, liveness, dataflow

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

## Reads through a computed name

A variable read through a name Tcl computes at run time counts as a use, even
though no `$name` token spells it:

```tcl
proc dump {} {
    set alpha 10           ;# not flagged — the loop below reads it
    set beta 20
    foreach v [info locals] { puts [set $v] }
}
```

Once a proc contains such a read — `[set $v]`, the double-`subst`
`[subst $[subst $v]]` idiom, or a `subst` over a template held in a variable —
`W211` goes silent for that whole proc, because no local in it can still be
proved unused. See
[W220](kcs-diagnostic-w220-dead-store.md#computed-variable-names-silence-the-check)
for the same rule on the dead-store side.

## Reads from a procedure you call

A procedure can run a script in its **caller's** frame, and that script can
read the caller's variables:

```tcl
proc runner {script} { uplevel 1 $script }
proc host {expr} {
    set threshold 10       ;# not flagged — `$script` may read it
    runner $expr
}
```

When the script is not readable, the callee could read any of the caller's
variables, so `W211` goes silent for the whole calling procedure. Which frame
the script runs in decides whose variables are protected: `uplevel 1 $script`
protects the *caller's*, `eval $script` protects the procedure that writes it.
See
[W220](kcs-diagnostic-w220-dead-store.md#a-procedure-you-call-can-read-your-variables)
for the same rule on the dead-store side.

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
