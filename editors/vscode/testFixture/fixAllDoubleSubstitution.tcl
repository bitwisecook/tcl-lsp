# "Fix All Safe Issues" must leave this file alone (issue #1195).
#
# Under C Tcl 9.0.3 this prints 5: the unbraced `expr` substitutes `$a` to
# the string `$x`, and `expr` then substitutes *that* to 3. W100's brace fix
# rewrites it to `expr {$a + $b}`, which errors because `a` holds the literal
# text `$x`. Bracing is still offered as its own named code action; it is not
# an unattended bulk rewrite.
set a {$x}
set x 3
set b 2
puts [expr $a + $b]
