# Conditional expression results

`normal_results.tcl` evaluates original expression, alias and container paths
on C Tcl 8.4, 8.5, 8.6, 9.0, 9.1 and current Jim. All engines return `3.5`,
`14` and `14`. The C 8.6–9.1 representation command observes Double, Int and
Int before any output formatting. C 8.4, C 8.5 and Jim have no such observer
in these binaries; their files retain values and observer unavailability.

`operators_and_shared_pool.tcl` covers integer negative powers, integer and
floating division, modulo rejection, and a constant expression result reused
after `llength`. C 8.5–9.1 and Jim return integer zero for integer `2 ** -1`;
C 8.4 rejects the operator. Floating modulo fails. C 8.6–9.1 expose a List
primary for the reused constant expression result after `llength`.

Conditional expression grammar, numeric category, current pool representation,
actual compiler preparation and actual result headers are independent facts.
A source analysis can describe operand normalisation or a fresh authored
numeric pool only with the corresponding original receipt. Unknown callbacks,
shared representation getters and numeric command-name cache conversion
withdraw that current pool evidence. Conditional templates supply no actual
entry, completed result or physical numeric object. Conditional grammar alone
cannot justify an executable rewrite.

`normalisation.tcl` compares nested expression normalization with its single
arithmetic invocation across integer, floating, hexadecimal and invalid inputs.
Both forms retain each engine's result and completion code. Replacing the
expression dispatcher with a Tcl procedure exposes two calls in the original
and one in the rewrite, so unknown or observed dispatch cannot authorize removal.
The C normalization recipe requires the same original source/frame/world and
closed selected handler/compiler/observer alternatives. Jim's ordinary values
agree in this fixture; its eager expression topology supplies no C recipe.

`math_binding.tcl` records the actual expression-function binding policy. Creating
`::tcl::mathfunc::abs` changes the normal result from `3` to `99` on C Tcl
8.5–9.1, while C Tcl 8.4 and Jim retain their fixed-table result `3`. A checked
function spelling and argument topology therefore supply no actual function
registration or erasure authority.
