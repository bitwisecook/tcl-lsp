package require tcltest
package require tclopt
namespace eval ::nativeTest {
    namespace path ::tclopt
    namespace import ::tcltest::*
    test native-empty {empty Euclidean norm} -body {mp_enorm {}} -result 0.0
    test native-input-preserved {C kernels modify private copies, not shared Tcl inputs} -body {
        set r {2 0 1 3}
        set alias $r
        set result [mp_covar 2 $r 2 {0 1} 1e-14]
        list $r $alias [expr {[dict get $result r] ne $r}]
    } -result {{2 0 1 3} {2 0 1 3} 1}
}
