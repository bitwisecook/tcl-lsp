namespace eval ::__tcl_lsp_probe_2286_lab2286 {proc p {} {return OLD}}
set c ::__tcl_lsp_probe_2286_lab2286::p
set before [eval $c]
namespace delete ::__tcl_lsp_probe_2286_lab2286
namespace eval ::__tcl_lsp_probe_2286_lab2286 {proc p {} {return NEW}}
list $before [eval $c]