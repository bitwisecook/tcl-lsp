proc check {label script initial} {
    unset -nocomplain v
    if {$initial} {set v OLD}
    set code [catch $script value]
    set exists [info exists v]
    set stored ABSENT
    if {$exists} {set stored $v}
    puts [list $label $initial $code $value $exists $stored]
}
foreach initial {0 1} {
    check regexp {regexp x y v} $initial
    check regexp_nocase {regexp -nocase x Y v} $initial
    check regexp_expanded_plain {regexp -expanded x Y v} $initial
    check regexp_expanded_space {regexp -expanded {a b} ab v} $initial
    check scan_decimal {scan abc %d v} $initial
    check scan_hex {scan xyz %x v} $initial
    check scan_literal {scan b {a%d} v} $initial
    check scan_match {scan 42 %d v} $initial
}
proc create_matcher_alias {name target captured} {
    if {[llength [info commands alias]] != 0} {
        eval [concat [list alias $name $target] $captured]
    } else {
        eval [concat [list interp alias {} $name {} $target] $captured]
    }
}
puts [list alias_capability [llength [info commands alias]]]
create_matcher_alias matcher_alias regexp {x y}
check captured_alias {matcher_alias v} 0
check captured_alias {matcher_alias v} 1
rename matcher_alias moved_matcher
check moved_alias {moved_matcher v} 0
check moved_alias {moved_matcher v} 1
rename regexp stock_matcher
check moved_builtin {stock_matcher x y v} 0
check moved_builtin {stock_matcher x y v} 1
proc regexp args {upvar 1 [lindex $args end] output; set output CUSTOM; return 1}
check replaced_stock {regexp x y v} 0
check replaced_stock {regexp x y v} 1
namespace eval app {proc regexp args {upvar 1 [lindex $args end] output; set output TAIL; return 1}}
check custom_same_tail {::app::regexp x y v} 0
check custom_same_tail {::app::regexp x y v} 1
