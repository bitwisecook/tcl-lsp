# O103 (fold static/pure proc calls to their constant return) fixture.
#
# `quad` has no explicit `return` — its result is Tcl's "value of the last
# command executed" rule (the KCS O103 doc's own canonical example uses
# this exact shape) — and must still fold to a constant.
proc quad {n} { expr {$n * 4} }
set q [quad 5]

# `triple`'s body is moved onto the name `double` by the rename below, so
# a call to `double` must NOT fold to the *original* `double` proc's
# constant return (that would miscompile: the renamed-over name actually
# runs `triple`'s body at runtime). The whole-module scan is conservative
# (flow-insensitive, like the existing O129 builtin-fold trust gate), so
# it distrusts every call to a name any `rename` ever touches, not only
# the ones textually after it.
proc double {n} { expr {$n * 2} }
proc triple {n} { expr {$n * 3} }
rename triple double
set d [double 21]
