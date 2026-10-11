# Ordinary CLI value observations only. No object-header or refcount probes.
proc emit_report {tag value} {
    binary scan $value H* hex
    puts "VALUE	$tag	[string length $value]	$hex"
}
set ::reports {}
oo::class create RoleBase {
    constructor {} {lappend ::reports [list ctor-base [self method]]}
    destructor {lappend ::reports [list dtor-base [self method]]}
}
oo::class create RoleDerived {
    superclass RoleBase
    constructor {} {
        lappend ::reports [list ctor-derived [self method]]
        next
    }
    destructor {
        lappend ::reports [list dtor-derived [self method]]
        next
    }
    method <constructor> {} {
        lappend ::reports [list named-literal [self method]]
    }
    method active {} {
        oo::define [self class] renamemethod active moved
        lappend ::reports [list active-renamed [self method]]
    }
    export <constructor>
}
set object [RoleDerived new]
$object <constructor>
$object active
$object destroy
foreach report $::reports {
    emit_report [lindex $report 0] [lindex $report 1]
}
puts "VALUES_DONE	[llength $::reports]"
