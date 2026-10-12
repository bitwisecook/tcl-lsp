source {/workspace/tcl-lsp/tmp/tcl8.6.18/library/tcltest/tcltest.tcl}
set argv [list -singleproc 1 -file var.test -relateddir {} -verbose {body error start skip}]
set code [catch {source {/workspace/tcl-lsp/tmp/tcl8.6.18/tests/all.tcl}} value]
puts [list outcome $code $value]
if {$code} {puts [list errorcode $::errorCode]; puts [list errorinfo $::errorInfo]}
