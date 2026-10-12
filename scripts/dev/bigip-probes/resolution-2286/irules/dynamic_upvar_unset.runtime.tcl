proc __tcl_lsp_probe_2286_lab2286_inner {n} {upvar 1 $n a; unset a; set a AGAIN}
proc __tcl_lsp_probe_2286_lab2286_outer {} {set x 1; __tcl_lsp_probe_2286_lab2286_inner x; return $x}
__tcl_lsp_probe_2286_lab2286_outer