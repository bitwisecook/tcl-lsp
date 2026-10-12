set events {}
oo::class create C {
    property a -get {return A} -set {
        lappend ::events a
        oo::define C {property b -get {return B} -set {lappend ::events b}}
    }
    property z -get {return Z} -set {lappend ::events z}
}
C create o
set code [catch {o configure -a ONE -z TWO} result options]
puts [list code $code result $result events $events properties [info object properties o -all -writable]]
