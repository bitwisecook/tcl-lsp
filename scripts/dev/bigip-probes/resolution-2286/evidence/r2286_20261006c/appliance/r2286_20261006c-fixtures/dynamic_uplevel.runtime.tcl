proc __tcl_lsp_probe_2286_r2286_20261006c_inner {} {uplevel 1 {set x 42}}
proc __tcl_lsp_probe_2286_r2286_20261006c_outer {} {set x 1; __tcl_lsp_probe_2286_r2286_20261006c_inner; return $x}
__tcl_lsp_probe_2286_r2286_20261006c_outer