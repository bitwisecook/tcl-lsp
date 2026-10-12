proc __tcl_lsp_probe_2286_lab2286_one {} {return OLD}
set c __tcl_lsp_probe_2286_lab2286_one
set before [eval $c]
rename __tcl_lsp_probe_2286_lab2286_one __tcl_lsp_probe_2286_lab2286_moved
proc __tcl_lsp_probe_2286_lab2286_one {} {return NEW}
list $before [eval $c] [__tcl_lsp_probe_2286_lab2286_moved]