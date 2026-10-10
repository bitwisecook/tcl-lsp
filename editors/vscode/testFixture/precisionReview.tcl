# Fixture for diagnostic ranges, rename
# resolution, and did-you-mean suggestions. Line numbers are load-bearing —
# the companion test (precisionReview.test.ts) asserts on them.
proc user_args {args} { puts [llength $args] }
rename user_args ua2
ua2 1 2
user_args 9
proc read_missing {} {set x [lindex $missing 0]; puts $x}
read_missing
proc greet {name} { puts $name }
greet a b
oo::class create Animal {
    method speak {} { return woof }
}
set pet [Animal new]
$pet spek
