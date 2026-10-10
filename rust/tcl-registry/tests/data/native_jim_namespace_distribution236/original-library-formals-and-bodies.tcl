if {[catch {info body {namespace ensemble}}]} {
    list NOT_APPLICABLE
} else {
    list [info args {namespace ensemble}] [info body {namespace ensemble}] [info args ensemble] [info body ensemble]
}
