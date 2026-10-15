puts [list PATCHLEVEL [info patchlevel]]
foreach {label lhs} {LEADING_ZERO 01 SPACED { 1 } EXPONENT 1e0 EXACT 1} {
    set numeric [expr {$lhs == 1}]
    set textual [expr {$lhs eq "1"}]
    switch -exact -- $lhs {1 {set exact 1} default {set exact 0}}
    puts [list $label $lhs NUMERIC $numeric TEXTUAL $textual EXACT $exact]
}
set lhs {a||b}
puts [list LITERAL_OR [expr {$lhs eq "a||b" || $lhs eq "c"}]]
set lhs {$value}
puts [list LITERAL_SIGIL [expr {$lhs eq {$value}}]]
set code [catch {if {{"$lhs eq a"}} {set result TRUE}} result]
puts [list BRACED_LITERAL_CONDITION $code $result]
