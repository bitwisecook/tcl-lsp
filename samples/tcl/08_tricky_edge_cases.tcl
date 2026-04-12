# 08: Tricky parsing situations

set lines [list \
    "alpha" \
    "beta" \
    "gamma"]

set result [join [lmap x $lines {string toupper $x}] ","]

switch -- $result {
    "ALPHA,BETA,GAMMA" {
        puts "match"
    }
    default {
        # Nested command substitution inside this body
        puts "no match: [clock seconds]"
    }
}

set complex "prefix-[string map {, _} $result]-[expr {[llength $lines] * 2}]"
puts $complex
