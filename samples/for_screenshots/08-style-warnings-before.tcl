set a "alpha"
set b "beta"

# Intentionally warning-prone patterns
if $a {
    puts "non-empty"
}
set eq_result [expr "$a == $b"]

puts $eq_result
#  ^--- cursor
