puts [list PROVIDER [info patchlevel]]
namespace eval ::Outer {
    oo::class create C
    oo::objdefine C export createWithNamespace
    namespace eval allocated {}
    set operand "allocated\0ignored"
    set status [catch {C createWithNamespace object $operand} value]
    if {$status} {set tuple $::errorCode} else {set tuple {}}
    puts [list RESULT $status $value $tuple [info commands object]]
}
