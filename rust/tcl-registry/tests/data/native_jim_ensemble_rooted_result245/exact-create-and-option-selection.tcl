if {[catch {info body {namespace ensemble}}]} {
    list NOT_APPLICABLE
} else {
    namespace eval ::N {
        set prefix_code [catch {namespace ensemble cre} prefix_result]
        set option_code [catch {namespace ensemble create -auto ::N::} option_result]
        list $prefix_code $prefix_result $option_code $option_result
    }
}
