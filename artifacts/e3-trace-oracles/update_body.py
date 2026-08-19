p = "/tmp/claude-0/-home-user-tcl-lsp/49cadaef-dfc0-5f80-9bf3-267da5a707d9/scratchpad/e3/pr-body.md"
s = open(p).read()

s = s.replace("## Mutation verification — 25 of 25 killed",
              "## Mutation verification — 26 of 26 killed", 1)

# New row for the review-round guard.
old_row = "| runtime ns-cmd flat reverse | `namespace_teardown_groups_interleaved_command_traces` |"
new_row = (old_row
           + "\n| runtime teardown drops the old-style flag | "
             "`legacy_letter_survives_namespace_teardown` |")
assert s.count(old_row) == 1
s = s.replace(old_row, new_row, 1)

# Record the review round alongside the adversary round.
anchor = "## Mutation verification"
review = """## Review round (Codex, two P2s)

**The old-style letter was lost through namespace teardown.** Teardown reduces
its collected callbacks to a name/prefix pair, dropping the flag, then always
appended `unset` — so an 8.x `trace variable ::ns::x u cb` fired by
`namespace delete` called back with `unset` where tclsh 8.6.16 says `u`. The
explicit-unset path was already correct, so the two disagreed with each other.
This is the single path where #1444's letter convention meets the #1440
teardown work, and neither side's tests covered it.

| case | tclsh 8.6.16 | before |
|---|---|---|
| legacy, fired by `namespace delete` | `L:u` | `L:unset` |
| modern, same teardown | `M:unset` | `M:unset` |
| both on one variable, newest-first | `Mz:unset Lz:u` | `Mz:unset Lz:unset` |
| legacy on an array element | `A:u` | `A:unset` |
| legacy, fired by explicit `unset` (control) | `E:u` | `E:u` |

The collected-callback type splits in two: the variable one carries the flag
and its firing site runs the op word through `callback_op_word`; the command
one does not, because all five `TCL_TRACE_OLD_STYLE` uses in `tclTrace.c` are
in the variable machinery and no command-trace equivalent exists. The grouping
helper became generic over the payload so both shapes still share it. Vectors
cover all four 8.x rows plus the 9.x side, where the legacy form does not exist
and teardown must still report `unset`.

**The `vinfo` spec detail promised the wrong result shape.** It still described
the modern `{opList command}` output; for 8.x the first element is the `rwua`
letter string. That text feeds hover, signature help, and completion, so it was
handing editors a false contract. Reworded to draw the distinction explicitly,
including the fixed r, w, u, a order — which is genuinely different from the
modern arm's `array read write unset`, and is the trap behind the original bug.

"""
assert s.count(anchor) == 1
s = s.replace(anchor, review + anchor, 1)
open(p, "w").write(s)
print("body updated")
