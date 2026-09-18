# Sample Tcl code exercising all optimisation passes.
# Used to demonstrate what each profile produces.
# Run: tcl opt --profile PROFILE samples/optimiser/input.tcl
#      (PROFILE is one of readability / standard / full / aggressive; each
#      committed profile_*.tcl carries this header, so follow it with that
#      file's own profile to reproduce it.)

# --- Readability candidates (O111, O114, O115, O117, O120) ---

# O114: incr idiom
set count 0
incr count
puts $count

# O117: string length check -> eq ""
proc is_empty {s} {
    if {$s eq ""} {
        return 1
    }
    return 0
}

# O120: eq/ne for string comparisons
proc greet {name} {
    if {$name eq "world"} {
        puts "Hello, world!"
    }
}

# O115: redundant nested expr.  O115 unwraps this `return` body — it is not
# limited to branch conditions.  Only `aggressive` rewrites it *here*, and the
# reason is elsewhere in this file: the `factorial` stanza below stops O115
# being reported at all in a single pass (see README.md).
proc double_expr {x} {
    return [expr {[expr {$x * 2}]}]
}

# --- Constant folding candidates (O100, O101, O102, O103, O110, O113, O116, O118) ---

proc add {a b} {
    return [expr {$a + $b}]
}

proc passthrough {x} {
    return $x
}

set timeout 30
set half [expr {$timeout / 2}]
set threshold [expr {$timeout + 10}]
set candidate [expr {$request_count + 1 + 2}]
set route [passthrough 42]

# O116: fold constant list
set colours [list red green blue]

# O118: fold constant lindex
set second [lindex {alpha beta gamma} 1]

# O113: strength reduction
proc square {r} {
    return [expr {$r ** 2}]
}

# --- Pattern recognition (O104, O119) ---

# O104: string build chain
proc build_banner {} {
    set msg {Hello World}
    return $msg
}

# O119: pack consecutive sets into lassign
proc init_vars {} {
    lassign {1 2 3} a b c
    list 1 2 3
}

# --- Dead code elimination (O107, O108, O109, O112, O126) ---

# O109: dead store

set stale 2
puts 2

# O126: unused variable
set unused_var [clock seconds]

# O107 + O112: unreachable code + constant condition

puts always

# O108: aggressive DCE


set rolling 5
puts 5

# --- Code motion (O106, O125, O127) ---

# O127: inline single-use variable
proc format_name {first last} {
    set full "$first $last"
    return $full
}

# --- Recursion transforms (O121, O122, O123) ---

# O122: every self-call is in tail position and passes one argument per
# parameter, so the whole proc becomes a `while {1}` loop. Overlap selection
# prefers it over the per-site O121 `tailcall` rewrite covering the same
# range. See README.md and `tail_call_loop_conversion_o122`.
proc factorial {n {acc 1}} {
    while {1} {
        if {$n <= 1} {
            return $acc
        }
        lassign [list [expr {$n - 1}] [expr {$n * $acc}]] n acc
    }
}

# O123: non-tail recursion hint (accumulator candidate)
proc sum_list {lst} {
    if {[llength $lst] == 0} {
        return 0
    }
    return [expr {[lindex $lst 0] + [sum_list [lrange $lst 1 end]]}]
}


# -------------
# optimised: 24 rewrite(s)
# O102  Forward literal load of 'count' from its single reaching definition
# O114  Use incr instead of set/expr
# O117  Simplify string length zero-check
# O120  Use eq/ne for string comparison
# O102  Forward literal load of 'timeout' from its single reaching definition
# O102  Forward literal load of 'timeout' from its single reaching definition
# O104  Remove dead intermediate string write
# O104  Remove dead intermediate string write
# O104  Fold write-only string build chain
# O119  Remove packed set (moved to lassign)
# O119  Remove packed set (moved to lassign)
# O119  Pack set statements into lassign
# O102  Forward literal load of 'a' from its single reaching definition
# O102  Forward literal load of 'b' from its single reaching definition
# O102  Forward literal load of 'c' from its single reaching definition
# O109  Eliminate dead store
# O102  Forward literal load of 'stale' from its single reaching definition
# O112  Eliminate dead if (all conditions are always false)
# O108  Eliminate transitively dead code
# O102  Forward literal load of 'rolling' from its single reaching definition
# O109  Eliminate dead store
# O102  Forward literal load of 'rolling' from its single reaching definition
# O122  Convert tail-recursive 'factorial' to iterative loop
# O123  Proc 'sum_list' is a candidate for accumulator-style rewriting
