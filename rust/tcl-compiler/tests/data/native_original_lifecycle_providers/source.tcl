if {![llength [info commands ::oo::class]]} {
    puts {availability|TclOO unavailable}
    return
}
set ::side BEFORE
oo::class create Base {
    constructor {x} {set ::side $x}
    destructor {set ::side CLEAN}
}
oo::class create Child {superclass Base}
Child create inherited AFTER
puts [list superclass-normal $::side [info object class inherited]]
inherited destroy
puts [list superclass-destruction $::side]

set ::side BEFORE
oo::class create Mixin {constructor {x} {set ::side MIXIN}}
oo::class create Mixed {
    mixin Mixin
    constructor {x} {set ::side CHILD}
}
Mixed create mixed AFTER
puts [list mixin-precedence $::side [info object class mixed]]
mixed destroy

set ::side BEFORE
oo::class create Failing {
    constructor {} {error BOOM}
    destructor {set ::side CLEAN}
}
oo::class create FailedChild {superclass Failing}
set code [catch {FailedChild create failed} result]
puts [list superclass-failure $code $result $::side [llength [info commands failed]]]

set ::side BEFORE
oo::class create Own {
    superclass Base
    constructor {x} {set ::side OWN}
}
Own create own AFTER
puts [list local-precedence $::side [info object class own]]
own destroy
