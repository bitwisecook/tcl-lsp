set nulname [format "item(k%csuffix)" 0]
set alias_status [catch {upvar #0 root $nulname} alias_message]
set namespace_result [namespace eval P {
    set nulname [format "item(k%csuffix)" 0]
    set status [catch {variable $nulname value} message]
    list $status $message
}]
list $alias_status $alias_message $namespace_result
