source {/workspace/tcl-lsp/tmp/tcl8.6.18/library/tcltest/tcltest.tcl}
source {/workspace/tcl-lsp/tmp/tcl8.6.18/library/clock.tcl}
trace add execution ::tcl::clock::format enter {apply {{args} {puts stderr Q0_enter}}}
trace add execution ::tcl::clock::format leave {apply {{args} {puts stderr Q0_leave}}}
trace add execution ::tcl::clock::GetSystemTimeZone enter {apply {{args} {puts stderr Q1_enter}}}
trace add execution ::tcl::clock::GetSystemTimeZone leave {apply {{args} {puts stderr Q1_leave}}}
trace add execution ::tcl::clock::ReadZoneinfoFile enter {apply {{args} {puts stderr Q2_enter}}}
trace add execution ::tcl::clock::ReadZoneinfoFile leave {apply {{args} {puts stderr Q2_leave}}}
trace add execution ::tcl::clock::SetupTimeZone enter {apply {{args} {puts stderr Q3_enter}}}
trace add execution ::tcl::clock::SetupTimeZone leave {apply {{args} {puts stderr Q3_leave}}}
trace add execution binary enter {apply {{args} {puts stderr Q4_enter}}}
trace add execution binary leave {apply {{args} {puts stderr Q4_leave}}}
trace add execution string enter {apply {{args} {puts stderr Q5_enter}}}
trace add execution string leave {apply {{args} {puts stderr Q5_leave}}}
trace add execution read enter {apply {{args} {puts stderr Q6_enter}}}
trace add execution read leave {apply {{args} {puts stderr Q6_leave}}}
set argv [list -singleproc 1 -file var.test -relateddir {} -verbose {body error start skip}]
source {/workspace/tcl-lsp/tmp/tcl8.6.18/tests/all.tcl}
