set ::__tcl_lsp_probe_2286_r2286_20261006c_events {}
proc __tcl_lsp_probe_2286_r2286_20261006c_trace {n i op} {lappend ::__tcl_lsp_probe_2286_r2286_20261006c_events [list $n $i $op]}
set ::__tcl_lsp_probe_2286_r2286_20261006c_x 1
trace variable ::__tcl_lsp_probe_2286_r2286_20261006c_x rwu __tcl_lsp_probe_2286_r2286_20261006c_trace
set a [set ::__tcl_lsp_probe_2286_r2286_20261006c_x]
set ::__tcl_lsp_probe_2286_r2286_20261006c_x 2
unset ::__tcl_lsp_probe_2286_r2286_20261006c_x
set ::__tcl_lsp_probe_2286_r2286_20261006c_x 3
list $a $::__tcl_lsp_probe_2286_r2286_20261006c_events