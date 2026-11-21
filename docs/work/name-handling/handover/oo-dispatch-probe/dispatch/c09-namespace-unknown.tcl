namespace eval ::app {
    namespace unknown ::app::miss
    proc miss args { return [list ::app::caught] }
    proc caught args { return caught }
    proc run {} { return [nosuch] }
}
puts [::app::run]
