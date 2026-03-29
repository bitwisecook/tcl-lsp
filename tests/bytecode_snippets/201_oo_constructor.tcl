# Constructor with variable binding (oo-2.2 equivalent)
oo::class create testClass {
    constructor {} {
        global result
        lappend result "[self]->construct"
    }
    method bar {} {
        global result
        lappend result "[self]->bar"
    }
}
set result {}
[testClass create foo] bar
set result
