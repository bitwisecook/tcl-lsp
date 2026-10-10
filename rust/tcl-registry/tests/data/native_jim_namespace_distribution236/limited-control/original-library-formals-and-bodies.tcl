if {![info exists jim_version]} {
    list NOT_APPLICABLE
} else {
    list [info args {namespace ensemble}] [info body {namespace ensemble}] [info args ensemble] [info body ensemble]
}
