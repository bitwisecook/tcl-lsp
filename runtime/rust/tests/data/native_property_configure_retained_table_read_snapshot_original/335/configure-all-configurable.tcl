set events {}
oo::configurable create C {
    property a -get {
        lappend ::events a
        oo::define C {
            property b -get {lappend ::events b; return B}
            method unrelated {} {return unchanged}
        }
        return A
    }
    property z -get {lappend ::events z; return Z}
}
C create o
set code [catch {o configure} result options]
puts [list code $code result $result events $events properties [info object properties o -all -readable]]
