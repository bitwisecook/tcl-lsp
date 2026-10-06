proc probe {name params body actual} {
    set made [catch [list proc p $params $body] message]
    set called -
    if {$made == 0} {set called [catch [linsert $actual 0 p] message]}
    puts [list $name $made $called $message]
}
probe middle {a args b} {list $a $args $b} {A B C D}
probe renamed {{args rest}} {set rest} {A B}
probe trailing_default {{args VALUE}} {info locals} {A B}
probe optional_before_required {a {b B} c} {list $a $b $c} {ONE TWO}
probe qualified {n::x} {set n::x} {VALUE}
probe array {a(k)} {set a(k)} {VALUE}
probe empty {{{}}} {set {}} {VALUE}
probe duplicate {args args} {set args} {A B}
probe quoted {{"x"}} {info locals} {VALUE}
set target BEFORE
probe ref {&x} {set x AFTER} {target}
puts [list target $target]
probe ref_default {{&x target}} {info locals} {}
probe missing_ref {&x} {set x AFTER} {missing}
probe middle_short {a args b} {list $a $args $b} {A B}
