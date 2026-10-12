proc __tcl_lsp_probe_2286_r2286_20261006c_one {} {return OLD}
set c __tcl_lsp_probe_2286_r2286_20261006c_one
set before [eval $c]
rename __tcl_lsp_probe_2286_r2286_20261006c_one __tcl_lsp_probe_2286_r2286_20261006c_moved
proc __tcl_lsp_probe_2286_r2286_20261006c_one {} {return NEW}
list $before [eval $c] [__tcl_lsp_probe_2286_r2286_20261006c_moved]