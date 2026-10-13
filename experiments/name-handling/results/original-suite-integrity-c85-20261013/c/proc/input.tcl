if {[catch {interp debug {} -frame 1} frame_debug]} {puts [list NH_FRAME_DEBUG unavailable $frame_debug]} else {puts [list NH_FRAME_DEBUG enabled]}
source "/workspace/tcl-lsp/tmp/tcl8.5.19/library/tcltest/tcltest.tcl"
namespace import -force ::tcltest::*
puts [list NH_HARNESS library [info library] tcl [package present Tcl] tcltest [package present tcltest] loader [package ifneeded tcltest [package present tcltest]]]
if {[file normalize [info library]] ne [file normalize "/workspace/tcl-lsp/tmp/tcl8.5.19/library"]} {error {wrong upstream library}}
::tcltest::configure -verbose {body error start skip}
source "/workspace/tcl-lsp/tmp/tcl8.5.19/tests/proc.test"
