# KCS: What does "Fix All Safe Issues" actually apply?

> **Audience:** User
> **Type:** Q&A

## Applies to

all-editors, diagnostic, refactoring

## Question

Which quick fixes does **Fix All Safe Issues** apply, and why does it leave
some obvious-looking ones alone?

## Answer

It applies only the fixes the analyser has **proved** do not change what
your program does. Every quick fix carries a safety class, decided for that
one occurrence rather than for the diagnostic code as a whole, and the bulk
command takes the `semantics-equivalent` class and nothing else.

The four classes are:

| Class | What it means | In the bulk fix? |
|---|---|---|
| **Semantics-equivalent** | Same completion code, result, output, and side effects, for every input. | Yes |
| **Behaviour-hardening** | Removes a hazard, and changes behaviour in doing so. That change *is* the fix. | No |
| **Style-only** | Changes only the spelling a person reads. | No |
| **Requires review** | A suggestion whose correctness depends on something the analyser could not check. | No |

The command used to work from a list of diagnostic codes — `W100`, `W110`,
and a few others — and applied their first fix without checking anything.
That promised a guarantee it did not deliver, because the same code produces
an equivalent rewrite in one place and a behaviour-changing one in the next.
Under real Tcl 9:

```tcl
set a {$x}
set x 3
set b 2
puts [expr $a + $b]      ;# prints 5
```

`W100` offers to brace that expression. Braced, `$a` is no longer
substituted before `expr` parses it, so `expr {$a + $b}` sees the literal
string `$x` and raises an error. Bracing an expression is still the right
thing to do — it is faster and it closes an injection hole — but it is a
change you should read before accepting, not one a bulk command should make
across a file. So `W100` is classified per occurrence: bracing
`expr 1 + 2` is equivalent and is applied, bracing `expr $a + $b` is
hardening and is not.

`W110` is the same story. `expr {"1" == "01"}` is `1` (a numeric
comparison) and `expr {"1" eq "01"}` is `0` (a string comparison), so
swapping the operator changes the answer in exactly the cases the diagnostic
is about.

Fixes that are not bulk-applicable are **not hidden**. Every one of them is
still offered individually in the lightbulb menu, with its own title, so you
can read the change and accept it deliberately.

Between passes, the command re-analyses the rewritten source. A fix whose
proof depended on text an earlier pass changed is therefore re-derived from
what is now there, rather than applied against stale evidence. Fixes whose
edits overlap are never applied together: the earlier one wins, and the
other is reconsidered on the next pass.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [How do I suppress a diagnostic?](kcs-howto-suppress-diagnostics.md)
- [W302 — catch without result](codes/kcs-diagnostic-w302-catch-without-result.md)
