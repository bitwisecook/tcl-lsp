proc __tcl_lsp_probe_2286_lab2286_inner {} {uplevel 1 {set x 42}}
proc __tcl_lsp_probe_2286_lab2286_outer {} {set x 1; __tcl_lsp_probe_2286_lab2286_inner; return $x}
__tcl_lsp_probe_2286_lab2286_outer