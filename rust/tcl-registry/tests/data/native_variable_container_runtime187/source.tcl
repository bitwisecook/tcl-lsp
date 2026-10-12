if {[catch {info patchlevel} probeVersion]} {
    set probeVersion unavailable
}
puts [list PATCHLEVEL $probeVersion]
foreach {label script} {
    update_scalar {set d {first NEW};set ok BEFORE;set entered 0;set c [catch {dict update d first ok {set entered 1}} r];list $c $r $ok $entered $d}
    update_array_second {array set blocked {k OLD};set d {first NEW second OTHER};set ok BEFORE;set entered 0;set c [catch {dict update d first ok second blocked {set entered 1}} r];list $c $r $ok $entered [catch {set blocked} v] $v $d}
    alias_array_replacement {array set aliasSource {k OLD};proc probeAliasWrite {} {upvar 1 aliasSource original;set original OTHER};set c [catch {probeAliasWrite} r];list $c $r [array exists aliasSource] [catch {set aliasSource} v] $v}
    reference_formal {set referenceSource {first NEW};proc probeReference {&original} {list $original};set c [catch {probeReference referenceSource} r];list $c $r $referenceSource}
    update_direct_multiword {set directSource {first DIRECT};set directTarget BEFORE;set entered 0;set c [catch {{dict update} directSource first directTarget {set entered 1}} r];list $c $r $directTarget $entered $directSource}
} {
    set probeCode [catch $script probeResult]
    puts [list RESULT $label $probeCode $probeResult]
}
