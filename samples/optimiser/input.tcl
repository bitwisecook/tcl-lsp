# Sample Tcl code exercising all optimisation passes.
# Used to demonstrate what each profile produces.
# Run: tcl opt samples/optimiser/input.tcl

# --- Readability candidates (O111, O114, O115, O117, O120) ---

# O114: incr idiom
set count 0
set count [expr {$count + 1}]

# O117: string length check -> eq ""
proc is_empty {s} {
    if {[string length $s] == 0} {
        return 1
    }
    return 0
}

# O120: eq/ne for string comparisons
proc greet {name} {
    if {$name == "world"} {
        puts "Hello, world!"
    }
}

# O115: redundant nested expr
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
    set msg {Hello}
    append msg { }
    append msg World
    return $msg
}

# O119: pack consecutive sets into lassign
proc init_vars {} {
    set a 1
    set b 2
    set c 3
    list $a $b $c
}

# --- Dead code elimination (O107, O108, O109, O112, O126) ---

# O109: dead store
set stale 1
set stale 2
puts $stale

# O126: unused variable
set unused_var [clock seconds]

# O107 + O112: unreachable code + constant condition
if {0} {
    puts "never reached"
    set never_seen 1
}
puts always

# O108: aggressive DCE
set rolling 1
set rolling [expr {$rolling + 1}]
set rolling 5
puts $rolling

# --- Code motion (O106, O125, O127) ---

# O127: inline single-use variable
proc format_name {first last} {
    set full "$first $last"
    return $full
}

# --- Recursion transforms (O121, O122, O123) ---

# O121 + O122: tail-recursive proc -> tailcall -> while loop
proc factorial {n {acc 1}} {
    if {$n <= 1} {
        return $acc
    }
    return [factorial [expr {$n - 1}] [expr {$n * $acc}]]
}

# O123: non-tail recursion hint (accumulator candidate)
proc sum_list {lst} {
    if {[llength $lst] == 0} {
        return 0
    }
    return [expr {[lindex $lst 0] + [sum_list [lrange $lst 1 end]]}]
}
