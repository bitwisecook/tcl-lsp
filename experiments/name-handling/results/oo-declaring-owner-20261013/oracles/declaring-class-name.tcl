if {[catch {info patchlevel} version]} {set version [info version]}
puts [list version $version]
if {[catch {info commands ::oo::class} available] || $available eq ""} {
    puts {unsupported TclOO}
    exit 0
}
namespace eval A {}
oo::class create A::B {
    method c {} {::oo::Helpers::self class}
}
oo::class create A {
    method B::c {} {::oo::Helpers::self class}
    export B::c
}
set left [A::B new]
set right [A new]
puts [list collision [$left c] [$right B::c]]
namespace eval ::N: {
    oo::class create C {method m {} {::oo::Helpers::self class}}
    set receiver [C new]
    puts [list colonNamespace [$receiver m]]
}
oo::class create :C {method m {} {::oo::Helpers::self class}}
set oneColon [:C new]
puts [list oneColon [$oneColon m]]
oo::class create Parent {method inherited {} {::oo::Helpers::self class}}
oo::class create Child {superclass Parent}
set derived [Child new]
puts [list inheritance [$derived inherited]]
rename Parent Moved
puts [list moved [$derived inherited]]
oo::objdefine $derived {method own {} {::oo::Helpers::self class}}
puts [list ownObject [catch {$derived own} message] $message]
oo::class create Constructor {constructor {} {puts [list constructor [::oo::Helpers::self class]]}}
Constructor new
