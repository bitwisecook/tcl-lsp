proc canonicalPrivateZero {} {::tcl::namespace::current}
proc canonicalPrivateExtra {} {::tcl::namespace::current [incr ::operandEffect]}
proc widePrivateZero {} {::::tcl:::namespace:::current}
proc widePrivateExtra {} {::::tcl:::namespace:::current [incr ::operandEffect]}
proc publicExtra {} {namespace current [incr ::operandEffect]}
foreach procedure {canonicalPrivateZero canonicalPrivateExtra widePrivateZero widePrivateExtra publicExtra} {
    set ::operandEffect 0
    set code [catch {$procedure} value]
    puts [list original $procedure $code $value $::operandEffect]
}
set renamed [catch {rename ::tcl::namespace::current ::renamedNamespaceWorker} renameValue]
puts [list rename-admission $renamed $renameValue]
if {!$renamed} {
    proc renamedPrivateZero {} {::renamedNamespaceWorker}
    proc renamedPrivateExtra {} {::renamedNamespaceWorker [incr ::operandEffect]}
    foreach procedure {renamedPrivateZero renamedPrivateExtra} {
        set ::operandEffect 0
        set code [catch {$procedure} value]
        puts [list renamed $procedure $code $value $::operandEffect]
    }
}
