set ::x GLOBAL
namespace eval ::N {
    proc observe {} {
        set x LOCAL
        if {[catch {interp create} child]} {
            set child [interp]
            set handle_api 1
            $child alias readX set x
            $child alias where namespace current
        } else {
            set handle_api 0
            interp alias $child readX {} set x
            interp alias $child where {} namespace current
        }
        set result [list [$child eval {readX}] [$child eval {where}]]
        if {$handle_api} {$child delete} else {interp delete $child}
        return $result
    }
}
puts [::N::observe]
