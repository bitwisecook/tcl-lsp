# 05: Intentionally warning-prone patterns
# Useful for testing diagnostics output.

set a foo
set b bar

# Unbraced if expression (often warned by linters/analysers)
if $a {puts "non-empty"}

# String comparison in expr using == (often flagged in favor of eq/ne)
if {$a == "foo"} {puts "matched"}
set eq_result [expr "$a == $b"]

# append with a space-separated value where lappend may be preferable
set list_like ""
append list_like "one two"

# Manual path concatenation instead of file join
set path "/tmp/"$a"/data.txt"
puts $path
