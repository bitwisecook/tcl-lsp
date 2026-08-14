package require tcl_lsp_fixture

set input 3
::tcl_lsp_fixture::collect output {
    set doubled [expr {$input * 2}]
    lappend output $doubled
}
puts $output
