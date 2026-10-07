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

- The variable name is greyed out — the code carries the editor's
  "unnecessary" tag — with the hint-severity message "Variable 'result' is set
  but never used".

## Example that triggers it

```tcl
set result [expr {1 + 1}]
```

The analyser reports **`W211`** on `result`: nothing reads it.

## Fix

Remove the unused assignment, or use the variable:

```tcl
set result [expr {1 + 1}]
puts $result
```

## Existence checks and unset count as uses

A variable that is only checked with `info exists` or `array exists`, or only
unset, is used: the check's answer and whether the `unset` succeeds depend on
the assignment. That holds wherever the check or the `unset` sits — a command
of its own, an `if` condition, or a `[…]` substitution inside another command:

```tcl
proc drain {} {
    set pending 1          ;# not flagged — the unset below needs it
    puts [unset pending]
}
```

Without the assignment, `unset pending` raises `can't unset "pending": no such
variable`.

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

## A variable you hand a procedure by name

A variable you name to a procedure that links it with `upvar` counts as used
by the call, and so does the variable a parameter's default names when the call
leaves that argument out:

```tcl
proc bumpd {{name n}} {upvar 1 $name v; incr v}
proc p {} {
    set n 1               ;# not flagged — bumpd reads n through the link
    bumpd
    return 0
}
```

## A command the file does not define can read your variables

A plain variable in the top-level script is the global `::name`, which a
command the file does not define, a call whose command is computed and a call
inside the body of a `catch` can read, so a top-level variable set before one
is not unused:

```tcl
set h 6
source other.tcl      ;# runs in this frame and may read h: not flagged
```

The same holds for a procedure's local: such a command — one an autoloader or
the unknown handler brings in — reaches it through `upvar 1`, and a `source`
runs its file in the procedure's frame. A local set and never read with no
such call after it still draws `W211`.

A command a [stub](../kcs-howto-annotate-commands-with-stubs.md) declares is
such a command until the stub states what it does to the caller's variables. A
plain stub — every argument a value, name, pattern or channel, no flag but
`-pure` or `-unsafe` — with `-frame own` or `-frame none` reads none of them,
and one with `-frame caller` only sets them, as `argparse` does, so a local set
before such a call and never read still draws `W211`. See
[W220](kcs-diagnostic-w220-dead-store.md#a-command-the-file-does-not-define-can-read-your-variables)
for the dead-store side.

## How to suppress

Add `# noqa: W211` on the line **above** the offending command, or set
`tclLsp.diagnostics.W211` to `false` to turn the check off entirely.

## How to change its severity

If the default hint is too subtle (or too loud), re-level it without disabling
it. In VS Code settings:

```json
{ "tclLsp.diagnosticSeverity.W211": "warning" }
```

Accepted values are `"default"` (the analyser's own severity, and the setting's
default), `"error"`, `"warning"`, `"information"`, and `"hint"`. Any diagnostic
code can be re-levelled with `tclLsp.diagnosticSeverity.<CODE>`; this changes
only how the editor renders the diagnostic, never the analysis.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [liveness](../../GLOSSARY.md#liveness)
- Related codes: `W210`, `W214`, `W220`
