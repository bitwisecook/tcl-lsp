set static::__tcl_lsp_probe_2286_r2286_20261006c_events {}
proc __tcl_lsp_probe_2286_r2286_20261006c_trace {n i op} {lappend static::__tcl_lsp_probe_2286_r2286_20261006c_events [list $n $i $op]}
set static::__tcl_lsp_probe_2286_r2286_20261006c_x 1
trace variable static::__tcl_lsp_probe_2286_r2286_20261006c_x rwu __tcl_lsp_probe_2286_r2286_20261006c_trace
set a [set static::__tcl_lsp_probe_2286_r2286_20261006c_x]
unset static::__tcl_lsp_probe_2286_r2286_20261006c_x
set static::__tcl_lsp_probe_2286_r2286_20261006c_x 3
list $a $static::__tcl_lsp_probe_2286_r2286_20261006c_events