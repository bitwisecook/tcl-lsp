namespace eval ::__tcl_lsp_probe_2286_r2286_20261006c {proc p {} {return OLD}}
set c ::__tcl_lsp_probe_2286_r2286_20261006c::p
set before [eval $c]
namespace delete ::__tcl_lsp_probe_2286_r2286_20261006c
namespace eval ::__tcl_lsp_probe_2286_r2286_20261006c {proc p {} {return NEW}}
list $before [eval $c]