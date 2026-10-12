set ::__tcl_lsp_probe_2286_lab2286_events {}
proc __tcl_lsp_probe_2286_lab2286_trace {n i op} {lappend ::__tcl_lsp_probe_2286_lab2286_events [list $n $i $op]}
set ::__tcl_lsp_probe_2286_lab2286_arr(k) 1
trace variable ::__tcl_lsp_probe_2286_lab2286_arr rwu __tcl_lsp_probe_2286_lab2286_trace
proc __tcl_lsp_probe_2286_lab2286_read {} {upvar #0 ::__tcl_lsp_probe_2286_lab2286_arr(k) v; return $v}
list [__tcl_lsp_probe_2286_lab2286_read] $::__tcl_lsp_probe_2286_lab2286_events