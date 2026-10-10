if {[catch {::info body {namespace info}}]} {
    list NOT_APPLICABLE
} else {
    list [::info args {namespace info}] [::info body {namespace info}]
}
