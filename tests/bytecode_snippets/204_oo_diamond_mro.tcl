# Diamond inheritance MRO dispatch (oo-9.1)
oo::class create A {
    method test {} {lappend ::result A; return ok}
}
oo::class create B {
    superclass A
    method test {} {lappend ::result B; next}
}
oo::class create C {
    superclass A
    method test {} {lappend ::result C; next}
}
oo::class create D {
    superclass B C
    method test {} {lappend ::result D; next}
}
set result {}
lappend result [[D new] test]
set result
