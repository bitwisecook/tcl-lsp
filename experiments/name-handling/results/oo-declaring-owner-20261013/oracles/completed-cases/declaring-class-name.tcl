if {[catch {info patchlevel} version]} {set version [info version]}
puts [list version $version]
if {[catch {info commands ::oo::class} available] || $available eq ""} {
    puts {unsupported TclOO}
    exit 0
}
proc _case {label script} {
    set code [catch {uplevel #0 $script} value]
    puts [list $label $code $value]
}
_case collision {
    namespace eval A {}
    oo::class create A::B {method c {} {::oo::Helpers::self class}}
    oo::class create A {method B::c {} {::oo::Helpers::self class}; export B::c}
    set left [A::B new]
    set right [A new]
    list [$left c] [$right B::c]
}
_case colonNamespace {
    namespace eval ::N: {
        oo::class create C {method m {} {::oo::Helpers::self class}}
        set receiver [C new]
        $receiver m
    }
}
_case oneColon {
    oo::class create :C {method m {} {::oo::Helpers::self class}}
    set receiver [:C new]
    $receiver m
}
_case inheritance {
    oo::class create Parent {method inherited {} {::oo::Helpers::self class}}
    oo::class create Child {superclass Parent}
    set derived [Child new]
    $derived inherited
}
_case moved {
    rename Parent Moved
    $derived inherited
}
_case ownObject {
    oo::objdefine $derived {method own {} {::oo::Helpers::self class}}
    $derived own
}
_case constructor {
    oo::class create Constructor {constructor {} {set ::constructorResult [::oo::Helpers::self class]}}
    Constructor new
    set ::constructorResult
}
