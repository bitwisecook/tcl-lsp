proc __tcl_lsp_probe_2286_lab2286_inner {n} {upvar 1 $n a; set a 9}
proc __tcl_lsp_probe_2286_lab2286_outer {} {set x 1; __tcl_lsp_probe_2286_lab2286_inner x; return $x}
__tcl_lsp_probe_2286_lab2286_outer