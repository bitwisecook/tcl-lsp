set static::__tcl_lsp_probe_2286_lab2286_events {}
proc __tcl_lsp_probe_2286_lab2286_trace {n i op} {lappend static::__tcl_lsp_probe_2286_lab2286_events [list $n $i $op]}
set static::__tcl_lsp_probe_2286_lab2286_x 1
trace variable static::__tcl_lsp_probe_2286_lab2286_x rwu __tcl_lsp_probe_2286_lab2286_trace
set a [set static::__tcl_lsp_probe_2286_lab2286_x]
unset static::__tcl_lsp_probe_2286_lab2286_x
set static::__tcl_lsp_probe_2286_lab2286_x 3
list $a $static::__tcl_lsp_probe_2286_lab2286_events