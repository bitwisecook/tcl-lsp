puts NH_INIT
source {/workspace/tcl-lsp/tmp/tcl8.6.18/library/tcltest/tcltest.tcl}
puts NH_HARNESS
set argv [list -singleproc 1 -file var.test -match var-13.2 -relateddir {} -verbose {body error start skip}]
puts NH_ORCHESTRATION_ENTER
source {/workspace/tcl-lsp/tmp/tcl8.6.18/tests/all.tcl}
puts NH_ORCHESTRATION_RETURN
