# KCS: T100 — Why does the analyser warn about tainted data in a code-execution sink?

> **Audience:** User
> **Type:** Diagnostic

## Applies to

all-editors, diagnostic, taint

## Profiles

default

## Question

Why does the analyser flag user-controlled data flowing into `eval`, `uplevel`, `subst`, unbraced `expr`, `exec`, or a *direct, numeric-coercing* operand of braced `expr` — including one used directly in an `if`/`while`/`for` condition?

## Why

T100 covers two related taint-into-evaluation hazards:

1. **Code execution.** User-controlled input reaching `eval`,
   `uplevel`, **unbraced** `expr` (``expr $cmd + 1``), `exec`, or
   `subst` (unless called with `-nocommands`, or the Tcl 9.1 positive
   form with no `-commands` — see Fix below) is re-parsed as Tcl
   syntax and can execute arbitrary commands. This is the classic
   injection vector. Calling through an `interp alias` or a
   `rename`d name (`rename eval myEval; myEval $x`) is caught the
   same way as calling the sink directly.

2. **Numeric / type coercion.** A tainted value used as a *direct*
   operand of a numeric or boolean operator inside **braced** `expr`
   (``expr {$data + 1}``, and equally an `if {$data + 1 > 5} {…}` /
   `while` / `for` condition) is NOT re-parsed as code -- Tcl
   evaluates the expression once with ``$data`` as a single operand.
   But the value still flows through Tcl's numeric coercion:
   ``"inf"`` returns inf, ``"0xff"`` parses as 255 even if base-10
   was intended, ``"0/0"`` raises a domain error. Decisions taken on
   the result can be subverted without any arbitrary-code execution.
   A tainted operand of a **pure string/list** operator (`eq`, `ne`,
   `lt`/`le`/`gt`/`ge`, `in`/`ni`, or the iRules
   `contains`/`starts_with`/`ends_with`/`equals`/`matches_glob`/
   `matches_regex` forms) never coerces, so T100 does not fire for
   it — `expr {$data eq "admin"}` is not a T100 hazard.

   For the same reason `switch` never raises T100 on its arms.
   `switch`, in every mode it has (`--`/`-exact`, `-glob`,
   `-regexp`), compares its subject as text — Tcl gives it no
   numeric-matching option — so `switch -- $cmd {status {…} version
   {…}}` on a tainted `$cmd` carries no coercion hazard and is not
   flagged.

The same code (T100) covers both because the underlying defence is
identical: validate / pin the value's shape *before* it reaches the
expression.  The emitted diagnostic message names the specific
hazard ("expr operand: numeric coercion" vs "eval: code injection")
so the appropriate remediation is obvious. The squiggle is drawn as
tightly as the analyser can manage: the one tainted argument word for
a command sink (`eval "prefix" $x "suffix"` underlines only `$x`),
the tainted operand itself for a braced `expr {…}` statement, or the
`{…}` condition text (not the whole `if`/`while`/`for` statement) for
a branch condition.

## Symptoms

- A yellow squiggle appears under the tainted argument or operand,
  with one of:
  - "Tainted variable ${var} flows into eval; possible code injection"
  - "Tainted variable ${var} flows into expr operand; numeric
    coercion may misinterpret value (use Tcl numeric-validation
    guards)"

## Example that triggers it

```tcl
set cmd [gets stdin]
eval $cmd
```

The analyser reports **`T100`** because `cmd` carries tainted data into `eval`.

```tcl
set n [gets stdin]
if {$n > 200} {
    puts big
}
```

The analyser also reports **`T100`** here: `$n` is a direct numeric
operand of `>` inside the `if` condition, evaluated exactly like any
other braced `expr` — this is not limited to a bare `expr` statement.
Wrapping the value in a command (`[string length $n]`) is not a direct
operand and does not fire.

## Fix

Dispatch on the value instead of evaluating it, using a string operator
that never coerces:

```tcl
proc report_status {} { puts ok }
proc report_version {} { puts 1 }

set cmd [gets stdin]
if {$cmd eq "status"} {
    report_status
} elseif {$cmd eq "version"} {
    report_version
}
```

For the numeric case, parse the value into one you produced yourself
rather than guarding around the tainted one:

```tcl
set line [gets stdin]
if {[scan $line %d n] == 1 && $n > 200} { puts big }
```

A `[string is integer -strict $n]` guard reads well but does not clear the
finding: the operand is still the tainted value.

For `subst`, when only variable and backslash substitution is needed, add
`-nocommands`. That removes the code-execution hazard outright, and the
analyser recognises it: `subst -nocommands $template` does not raise T100.
A quick fix ("Add -nocommands to disable command substitution") is offered
on a `subst` sink.

## How to suppress

Add `# noqa: T100` on the line **above** the offending command.

## Related

- [KCS codes index](README.md)
- [Diagnostics feature](../features/kcs-feature-diagnostics.md)
- [taint analysis](../../GLOSSARY.md#taint-analysis)
- Related codes: `T101`, `T102`, `W300`
