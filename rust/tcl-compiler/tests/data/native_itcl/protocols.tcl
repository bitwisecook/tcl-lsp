# tcl-lsp — a language server and toolchain for Tcl
# SPDX-License-Identifier: AGPL-3.0-or-later
# Run with the measured C Tcl 8.6.18/9.0.4/9.1.0 Itcl 4.3.2 build.
# Each case uses a fresh interpreter.
if {[llength $argv] != 2} {error "usage: protocols.tcl ITCL_LIBRARY ITCL_SO"}
foreach {label script} {
    named {itcl::class C {method ping {} {return pong}}; set o [C named]; list $o [$o ping] [info commands named]}
    generated {itcl::class C {}; C #auto}
    class_proc {itcl::class C {proc ping {} {return static}; method status {} {return instance}}; set o [C ping]; list $o [$o status] [C::ping]}
    retired {itcl::class C {constructor {} {rename $this {}; return}}; set o [C named]; list $o [info commands named]}
    constructor_error {itcl::class C {constructor {} {error BOOM}}; list [catch {C named} value] $value [info commands named]}
    metaclass_replaced {oo::define ::itcl::clazz method unknown args {return hijacked}; itcl::class C {}; list [C named] [info commands named]}
    private_replaced {rename ::itcl::parser::method ::itcl::parser::saved; proc ::itcl::parser::method args {}; itcl::class C {method ping {} {return pong}}; set o [C named]; list [catch {$o ping}]}
    parser_observed {proc observe args {error PRIVATE_TRACE}; trace add execution ::itcl::parser::method enter observe; list [catch {itcl::class C {method ping {} {return pong}}} value] $value [info commands C]}
    qualified {namespace eval N {itcl::class C {method ping {} {return $this}}; set o [C child]; list $o [$o ping]}}
} {
    set child [interp create]
    interp eval $child [list set ::env(ITCL_LIBRARY) [lindex $argv 0]]
    interp eval $child [list load [lindex $argv 1] Itcl]
    set code [catch {interp eval $child $script} result]
    puts [list $label $code $result]
    interp delete $child
}
