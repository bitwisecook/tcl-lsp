# Destructor lifecycle (oo-3.3 equivalent)
oo::class create foo {
    constructor {} {lappend ::result made}
    destructor {lappend ::result died}
}
set result {}
set obj [foo new]
$obj destroy
set result
