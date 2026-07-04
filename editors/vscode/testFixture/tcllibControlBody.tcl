package require control
package require struct::list

control::do {
    set total [expr {$total + 1}]
    puts $total
} while {$total < 10}

control::assert {$total == 10}

struct::list foreachperm perm {a b c} {
    puts $perm
}
