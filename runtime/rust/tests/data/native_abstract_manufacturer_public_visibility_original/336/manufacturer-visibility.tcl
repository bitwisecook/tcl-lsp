oo::abstract create Abstract {}
puts [list abstract_public [lsort [info object methods Abstract -all]]]
puts [list abstract_private [lsort [info object methods Abstract -all -private]]]
foreach method {new create createWithNamespace} {
    if {$method eq "new"} {set command [list Abstract new]}
    if {$method eq "create"} {set command [list Abstract create x]}
    if {$method eq "createWithNamespace"} {set command [list Abstract createWithNamespace x ::x]}
    set code [catch $command result]
    puts [list attempt $method code $code result $result]
}
oo::class create Concrete {superclass Abstract}
set code [catch {Concrete create c} result]
puts [list concrete_create code $code class [info object class c]]
oo::objdefine Abstract export new
set code [catch {Abstract new} result]
if {$code == 0} {puts [list explicit_export_new code $code class [info object class $result]]} else {puts [list explicit_export_new code $code result $result]}
puts [list abstract_public_after [lsort [info object methods Abstract -all]]]
