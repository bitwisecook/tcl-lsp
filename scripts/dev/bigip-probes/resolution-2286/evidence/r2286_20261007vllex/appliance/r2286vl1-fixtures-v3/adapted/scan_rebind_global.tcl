set ::__tcl_lsp_2286_vl1_replacement BEFORE
set first INIT
set second ORIGINAL
proc __tcl_lsp_2286_vl1_global_watch {n k op} {uplevel 1 {upvar #0 ::__tcl_lsp_2286_vl1_replacement second}}
trace variable first w __tcl_lsp_2286_vl1_global_watch
set n first
set m second
set h scan
set count [$h {7 9} {%d %d} $n $m]
list $count $first $second [set ::__tcl_lsp_2286_vl1_replacement]
