set ::errorCode SENTINEL
set v OLD
proc __tcl_lsp_2286_vl1_error_watch {n k op} {error BOOM}
trace variable v r __tcl_lsp_2286_vl1_error_watch
set h lappend
set answer [$h v]
trace vdelete v r __tcl_lsp_2286_vl1_error_watch
list $answer [set ::errorCode] [set v]
