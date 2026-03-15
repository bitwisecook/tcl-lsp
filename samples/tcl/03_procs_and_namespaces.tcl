# 03: Namespaces and procedure definitions

namespace eval ::math {
    proc add {a b} {
        return [expr {$a + $b}]
    }

    proc maybe_add {a {b 10}} {
        return [expr {$a + $b}]
    }
}

puts [::math::add 2 4]
puts [::math::maybe_add 7]
