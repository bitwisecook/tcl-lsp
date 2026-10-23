if {[catch {::info body {namespace info}}]} {
    list NOT_APPLICABLE
} else {
    proc {namespace info} args {list HELPER [uplevel 1 {namespace current}] $args}
    list [::info commands ::r2286*] [::info commands -all ::r2286*] [::info -nons commands ::r2286*]
}
