# next at end of chain raises error (oo-7.8)
oo::class create foo {
    method bar {} {lappend ::result foo; lappend ::result [next] foo}
}
oo::class create foo2 {
    superclass foo
    method bar {} {lappend ::result foo2; lappend ::result [next] foo2}
}
set result {}
lappend result [catch {[foo2 new] bar} msg] $msg
set result
