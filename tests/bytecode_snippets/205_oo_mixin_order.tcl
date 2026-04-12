# Class mixin dispatch order (oo-14.8 — Bug 1998221)
set ::result {}
oo::class create parent {
    method test {} {}
}
oo::class create mix {
    superclass parent
    method test {} {lappend ::result mix; next; return $::result}
}
oo::class create cls {
    superclass parent
    mixin mix
    method test {} {lappend ::result cls; next; return $::result}
}
[cls new] test
