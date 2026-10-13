if {[catch {info patchlevel} version]} {set version [info version]}
puts [list version $version]
if {[catch {info commands ::oo::class} available] || $available eq ""} {
    puts {unsupported TclOO}
    exit 0
}
namespace eval N {}
set code [catch {
    oo::class create N:::C {method name {} {::return [::oo::Helpers::self class]}}
    set qualified [N::C new]
    puts [list qualified [$qualified name]]
    rename N::C N::Moved
    puts [list renamed [$qualified name]]
} message]
puts [list qualified-status $code $message]
set code [catch {
    oo::class create :C {method name {} {::return [::oo::Helpers::self class]}}
    set leading [:C new]
    puts [list leading [$leading name]]
} message]
puts [list leading-status $code $message]
puts [list leading-class-remains [info object isa class :C]]
namespace eval N: {
    set code [catch {
        oo::class create C {method name {} {::return [::oo::Helpers::self class]}}
        set scoped [C new]
        puts [list entered [$scoped name]]
    } message]
    puts [list entered-status $code $message]
    puts [list entered-class-remains [info object isa class C]]
}
