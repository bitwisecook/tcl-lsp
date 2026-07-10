# TclOO constructor call-site arity (`ClassName new` / `ClassName create`)
# and direct `apply {{params} body}` lambda-call arity — previously neither
# produced any diagnostic at all, even with a plainly wrong argument count.

oo::class create Widget {
    constructor {a b} {
        return
    }
}

# Too few arguments for `new` — must fire E002.
Widget new 1

# `create`'s mandatory object-name word plus the constructor's own two
# arguments — three extra words is one too many, must fire E003.
Widget create fido 1 2 3

# A subclass with no constructor of its own inherits the superclass's —
# must still fire E002 here.
oo::class create Base {
    constructor {a b} {
        return
    }
}
oo::class create Sub {
    superclass Base
}
Sub new 1

# No explicit constructor anywhere in the hierarchy — TclOO's inherited
# default constructor accepts any argument count, so this must stay silent.
oo::class create NoCtor {
    method bar {} {
        return
    }
}
NoCtor new 1 2 3 4 5

# A direct call to an inline lambda — too few arguments, must fire E002.
apply {{a b} {return [expr {$a + $b}]}} 1

# A correctly-arg-counted constructor call and lambda call must stay silent.
Widget create ok1 1 2
apply {{a b} {return [expr {$a + $b}]}} 1 2
