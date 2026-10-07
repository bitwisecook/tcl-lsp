set events {}
set a(k) OLD
proc __tcl_lsp_2286_vl1_argument_watch {n k op} {
    upvar 1 events events
    binary scan $n H* name_hex
    binary scan $k H* index_hex
    lappend events [list $op $name_hex $index_hex]
    if {$op eq "read"} {uplevel 1 {unset a(k); set a(k) NEW}}
}
trace add variable a(k) {read unset} __tcl_lsp_2286_vl1_argument_watch
set h lappend
set answer [$h a(k) EXTRA]
set summary [list $answer [set a(k)] $events [trace info variable a(k)]]
set summary
