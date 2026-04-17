# Mixin of mixin method access (oo-14.6 — Bug 1960703)
oo::class create parent
oo::class create A {
    superclass parent
    method egg {} {
        return chicken
    }
}
oo::class create B {
    superclass parent
    mixin A
    method bar {} {
        my egg
    }
}
oo::class create C {
    superclass parent
    mixin B
    method foo {} {
        my bar
    }
}
[C new] foo
