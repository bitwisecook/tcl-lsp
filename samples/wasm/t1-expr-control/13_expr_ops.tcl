# T1: the operator surface expr must cover natively: comparison, logical,
# ternary, bitwise, shifts, string comparison, exponent, math functions.
set a 12
set b 5
puts [expr {$a > $b && $b > 0}]
puts [expr {$a == 12 ? "yes" : "no"}]
puts [expr {$a & $b}]
puts [expr {$a | $b}]
puts [expr {$a ^ $b}]
puts [expr {$a << 2}]
puts [expr {$a >> 1}]
puts [expr {~$a}]
puts [expr {!$b}]
puts [expr {$b ** 3}]
puts [expr {"abc" eq "abc"}]
puts [expr {"abc" ne "abd"}]
puts [expr {"abc" < "abd"}]
puts [expr {max($a, $b) + min($a, $b)}]
puts [expr {abs(-$a)}]
puts [expr {double($a) / $b}]
puts [expr {$a in {1 12 3}}]
