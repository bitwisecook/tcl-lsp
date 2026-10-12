puts NH_INIT
source {/workspace/tcl-lsp/tmp/tcl8.6.18/library/tcltest/tcltest.tcl}
package require tcltest 2.5
namespace import -force ::tcltest::*
puts [list NH_HARNESS library [info library] tcl [package present Tcl] tcltest [package present tcltest]]
if {[file normalize [info library]] ne [file normalize {/workspace/tcl-lsp/tmp/tcl8.6.18/library}]} {error {wrong upstream library}}
::tcltest::configure -singleproc 1 -verbose {body error start skip}
interp debug {} -frame 1
puts [list NH_CONSTRAINT singleProcess [::tcltest::singleProcess] singleTestInterp [::tcltest::testConstraint singleTestInterp]]
puts NH_SUITE_ENTER
source {/workspace/tcl-lsp/tmp/tcl8.6.18/tests/upvar.test}
puts NH_SUITE_RETURN
