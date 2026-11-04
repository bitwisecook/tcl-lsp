proc report {case iteration phase value} {
    if {[llength [info commands tcl::unsupported::representation]]} {
        puts [list [info patchlevel] $case $iteration $phase [tcl::unsupported::representation $value]]
    }
}
proc fresh_sequential {} {
    foreach iteration {1 2 3} {
        set x "s"
        report fresh-sequential $iteration literal $x
        set x [list 1]
        report fresh-sequential $iteration list $x
    }
}
proc dependent_int_string {} {
    set x 0
    foreach iteration {1 2 3} {
        report dependent-int-string $iteration entry $x
        set x [expr {$x + 1}]
        report dependent-int-string $iteration expr $x
        set x [string range $x 0 end]
        report dependent-int-string $iteration range $x
    }
}
proc same_object_int_list {} {
    set text abcdef
    set x [string length $text]
    foreach iteration {1 2 3} {
        report same-object-int-list $iteration entry $x
        lindex $x 0
        report same-object-int-list $iteration list-use $x
        expr {$x + 1}
        report same-object-int-list $iteration numeric-use $x
    }
}
proc conditional_accumulator {} {
    set ys {}
    foreach iteration {0 1 0 1} {
        report conditional-accumulator $iteration entry $ys
        if {$iteration} {lappend ys $iteration}
        report conditional-accumulator $iteration exit $ys
    }
}
proc dependent_list_string {} {
    set acc ""
    foreach iteration {1 2 3} {
        lappend acc $iteration
        report dependent-list-string $iteration list $acc
        set acc "$acc,"
        report dependent-list-string $iteration interpolation $acc
    }
}
proc fresh_branchy {} {
    foreach iteration {1 2 3} {
        if {$iteration % 2} {
            set x "value"
        } else {
            set x [list 1 2]
        }
        report fresh-branchy $iteration branch $x
    }
}
proc dependent_branchy {} {
    set x seed
    foreach iteration {1 2 3 4} {
        report dependent-branchy $iteration entry $x
        if {$iteration % 2} {
            set x [string range $x 0 end]
        } else {
            set x [list 1 2]
        }
        report dependent-branchy $iteration branch $x
    }
}
proc dependent_branchy_int_string {} {
    set x 0
    foreach iteration {1 2 3 4} {
        report dependent-branchy-int-string $iteration entry $x
        if {$iteration % 2} {
            set x [expr {$x + 1}]
        } else {
            set x [string range $x 0 end]
        }
        report dependent-branchy-int-string $iteration branch $x
    }
    return $x
}
fresh_sequential
dependent_int_string
dependent_branchy_int_string
same_object_int_list
conditional_accumulator
dependent_list_string
fresh_branchy
dependent_branchy
puts [list portable-complete [dependent_branchy_int_string]]
