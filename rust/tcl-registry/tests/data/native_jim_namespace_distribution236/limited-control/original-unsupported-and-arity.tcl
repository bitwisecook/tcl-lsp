if {![info exists jim_version]} {
    list NOT_APPLICABLE
} else {
    set root_code [catch {namespace ensemble create} root_result]
    set operation_code [catch {namespace ensemble exists ::N} operation_result]
    set option_code [catch {namespace eval ::N {namespace ensemble create -map {go ::list}}} option_result]
    list $root_code $root_result $operation_code $operation_result $option_code $option_result
}
