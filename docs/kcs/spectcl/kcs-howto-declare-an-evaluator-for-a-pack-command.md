# KCS: How do I declare an evaluator for my own pack command?

> **Audience:** User
> **Type:** How-To

## Applies to

all-editors, tcl-lsp-cli, mcp

## Question

I have described my own command as a SpecTcl pack. How do I also give it
a computed answer, so the analyser, `tcl diag`, `tcl opt`, and the
Compiler Explorer see what a call returns and what it writes, instead of
treating every call as unknown?

## Before you start

- You have already written the command's `.tclspec` declaration — see
  [How do I write a SpecTcl pack?](../kcs-howto-write-a-tclspec-pack.md).
- Your command's own implementation is already loop-free Tcl over a small,
  fixed set of built-ins. An evaluator *models* an existing
  implementation; it does not replace it, and no independent proof that
  the model matches is required — it is your claim, and the analyser
  trusts it the way it trusts the rest of a loaded pack.

## Answer

1. **Say what the call computes and writes, in `semantics { … }`.**
   `effects { no_store_writes no_external_io }` for a pure result;
   `result -semantic string` (or another kind) for what the answer
   represents; `stores -targets {N …} -outcome writes|may_write` for
   every argument index the command writes through.
2. **Give it a body, in `evaluate -implementation ID -host bounded_tcl { … }`.**
   Four rows, all required:
   - `inputs { arg N exact … target N incoming … option -NAME exact … }`
     — exactly what the body reads. Anything not listed here is never
     supplied to the body, and a body that reads a target's value without
     listing it here is a notice waiting for you in step 5. An
     `option -NAME exact` input arrives as a Tcl **list of zero or one
     element**, not a bare string: the empty list when `-NAME` was never
     written on the call, and a one-element list holding its value when
     it was written exactly once — so your body can tell "the caller
     never passed this" apart from "the caller passed an empty string".
     Writing `-NAME` twice on one call, or writing it as the very last
     word with nothing after it, has no answer at all.
   - `depends { tcl_profile implementation_identity … }` — what else the
     answer depends on, so a change to any of those invalidates it.
   - `budget { -commands N -wall-clock MS -value-bytes N }` — narrows the
     server's own limits for this call only; it can only make them
     smaller, never larger.
   - `body {params} { … }` — ordinary Tcl, one parameter per declared
     input, in order. Inside it: `fold VALUE` for the result, and `write
     TARGET VALUE` or `preserve TARGET` for each declared target, in call
     order. Say nothing about a declared target and the whole answer is
     dropped, not just that target — an accumulator you always touch on
     every path through the body is the way to avoid this.
   - Optional: `facts { … }` states finer-grained result and taint
     relationships (`result -string_segments {…}`, `taint -result_from
     {…}`) for the tools that read them.
3. **Write it as a pure function of its declared inputs.** The body runs
   in the same isolated sandbox every hook does, plus one more rule: a
   store to anything outside its own local variables — a `::`-qualified
   name, a namespace variable — is refused, and reading one the server's
   own bootstrap set (`::env`, `::tcl_platform`, and the like) is refused
   too, so the answer can never depend on the machine tcl-lsp happens to
   be analysing on. If your algorithm genuinely depends on something
   beyond its arguments, name it in `depends`; do not reach for a global.
4. **Know what makes an evaluator give up, and why each is not a "no".**
   Several separate things all mean "no answer for this call, not this
   time" rather than a wrong answer:
   - An argument the analyser has not proven a fixed value for yet, or
     one reached through `{*}` expansion — the body is never invoked
     with a placeholder.
   - A call missing a required argument, or two declared targets that
     turn out to be the same variable.
   - A workspace or dialect with no fixed Tcl release to run the body
     under — your body has no cross-release guarantee, so it only
     answers where a release is pinned.
   - The evaluator host is momentarily unavailable, or was switched off
     after an earlier crash or budget overrun on your own pack. This one
     is temporary: the very next analysis tries again.
   - The body itself raises an error, or finishes having said nothing
     about a declared target (step 2's last point).
5. **Check `mcp__tcl-lsp__spectcl_check` before you rely on any of this.**
   It flags an evaluator that reads a target it never declared, one that
   is silent on a target it did declare, and a `write` naming something
   outside the declared targets — the same three rules the server
   enforces when it actually answers a call, surfaced while you are still
   editing.

## How to tell it worked

Run `tcl explore --show sccp --text` over a small script that calls your
command with a literal argument, from a workspace where your pack loads.
The line for your call reads `route <name>: implementation <your-id>`
and `· answer: evaluated`, with the folded value shown on the statement
that uses it. The same fold shows up as a constant condition in `tcl diag`
(a branch on it becomes I230) and as a forwarded constant in `tcl opt`
(O100) wherever every input is known. Renaming the command, or adding a
second form with the operand in a different position, needs no change
anywhere else — only the declaration moves.

## Related

- [How do I write a SpecTcl pack?](../kcs-howto-write-a-tclspec-pack.md)
- [What does a command's value-transfer declaration tell the analyser?](../compiler/kcs-qa-what-does-a-value-transfer-declaration-say.md)
- [The evaluation contract](../../design/compiler/value-evaluation.md) — the full route, host, memo, and budget contract
- [SpecTcl pack design](../../design/registry/spec-packs.md)
- [KCS index](../README.md)
- [Glossary](../../GLOSSARY.md)
