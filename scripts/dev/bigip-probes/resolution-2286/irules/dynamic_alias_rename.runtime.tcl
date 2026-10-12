proc __tcl_lsp_probe_2286_lab2286_one {} {return ORIGINAL}
interp alias {} __tcl_lsp_probe_2286_lab2286_alias {} __tcl_lsp_probe_2286_lab2286_one
rename __tcl_lsp_probe_2286_lab2286_one __tcl_lsp_probe_2286_lab2286_moved
list [catch {__tcl_lsp_probe_2286_lab2286_alias} a] $a [__tcl_lsp_probe_2286_lab2286_moved]