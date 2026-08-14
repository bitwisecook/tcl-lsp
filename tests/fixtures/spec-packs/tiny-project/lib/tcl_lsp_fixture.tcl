# A deliberately small package used by the SpecTcl integration tests.
package provide tcl_lsp_fixture 1.0

namespace eval ::tcl_lsp_fixture {
    namespace export collect
}

# Evaluate script in the caller while exposing resultVar as a list accumulator.
#
# resultVar is the name of a variable in the caller. The command clears that
# variable before evaluating script, then returns its final value.
proc ::tcl_lsp_fixture::collect {resultVar script} {
    upvar 1 $resultVar result
    set result {}
    uplevel 1 $script
    return $result
}
