if {[catch {info patchlevel} version]} {set version [info version]}
puts [list version $version]
if {[catch {info commands ::oo::class} available] || $available eq ""} {
    puts {unsupported TclOO}
    exit 0
}
namespace eval N {}
oo::class create N:::C {method name {} {::return [::oo::Helpers::self class]}}
set qualified [N::C new]
puts [list qualified [$qualified name]]
oo::class create :C {method name {} {::return [::oo::Helpers::self class]}}
set leading [:C new]
puts [list leading [$leading name]]
namespace eval N: {
    oo::class create C {method name {} {::return [::oo::Helpers::self class]}}
    set scoped [C new]
    puts [list entered [$scoped name]]
}
rename N::C N::Moved
puts [list renamed [$qualified name]]
