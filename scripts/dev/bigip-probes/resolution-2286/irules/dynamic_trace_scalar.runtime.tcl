set ::__tcl_lsp_probe_2286_lab2286_events {}
proc __tcl_lsp_probe_2286_lab2286_trace {n i op} {lappend ::__tcl_lsp_probe_2286_lab2286_events [list $n $i $op]}
set ::__tcl_lsp_probe_2286_lab2286_x 1
trace variable ::__tcl_lsp_probe_2286_lab2286_x rwu __tcl_lsp_probe_2286_lab2286_trace
set a [set ::__tcl_lsp_probe_2286_lab2286_x]
set ::__tcl_lsp_probe_2286_lab2286_x 2
unset ::__tcl_lsp_probe_2286_lab2286_x
set ::__tcl_lsp_probe_2286_lab2286_x 3
list $a $::__tcl_lsp_probe_2286_lab2286_events