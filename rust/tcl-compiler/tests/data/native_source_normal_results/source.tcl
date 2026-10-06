proc observe {name script includeResult} {
    set code [catch {uplevel 1 $script} result]
    if {!$includeResult} {set result {}}
    puts [list $name $code $result]
}
observe final_set {proc f {} {set z 3}; f} 1
observe numeric_call {proc f {} {expr {double(21)}}; f} 1
observe caller_three {proc f {x} {set local $x}; f 3} 1
observe caller_four {f 4} 1
observe variadic_numeric {proc f {args} {expr {[lindex $args 0] * 2}}; f 5} 1
observe return_arm {proc f {flag} {if {$flag} {return 3}; set z 4}; f 1} 1
observe fallthrough_arm {f 0} 1
observe arithmetic_error {proc f {} {expr {1 / 0}}; f} 0
observe default_return {proc f {} {return 42}; f} 1
observe level_two_return {proc f {} {return -level 2 42}; f} 1
observe error_return {proc f {} {return -code error 42}; f} 1
observe unknown_prefix {proc f {} {notacommand; return 42}; f} 0
observe unrelated_rename {proc unrelated {} {return 99}; proc answer {} {return 42}; rename unrelated moved; answer} 1
namespace eval ::tcl::mathfunc {}
observe replaced_numeric_call {proc ::tcl::mathfunc::double args {error CUSTOM}; proc f {} {expr {double(21)}}; f} 1
