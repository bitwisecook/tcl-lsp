set ::__tcl_lsp_probe_2286_r2286_20261006c_events {}
proc __tcl_lsp_probe_2286_r2286_20261006c_trace {n i op} {lappend ::__tcl_lsp_probe_2286_r2286_20261006c_events [list $n $i $op]}
set ::__tcl_lsp_probe_2286_r2286_20261006c_arr(k) 1
trace variable ::__tcl_lsp_probe_2286_r2286_20261006c_arr rwu __tcl_lsp_probe_2286_r2286_20261006c_trace
proc __tcl_lsp_probe_2286_r2286_20261006c_read {} {upvar #0 ::__tcl_lsp_probe_2286_r2286_20261006c_arr(k) v; return $v}
list [__tcl_lsp_probe_2286_r2286_20261006c_read] $::__tcl_lsp_probe_2286_r2286_20261006c_events