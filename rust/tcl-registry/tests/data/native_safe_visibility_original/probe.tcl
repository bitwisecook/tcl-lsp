# ASCII source. Each case owns and deletes its child; outputs retain exact result bytes.
puts "PATCHLEVEL [info patchlevel]"
proc r2286_observe {label script} {
    set code [catch {uplevel 1 $script} result]
    binary scan $result H* hex
    puts "OBSERVATION $label $code $hex"
}
r2286_observe safe_colon_and_namespace {
    interp create -safe r2286_s
    set code [catch {
        interp eval r2286_s {
            proc :source {args} {return LITERAL_COLON}
            namespace eval : {proc source {args} {return COLON_NAMESPACE}}
            list [info commands source] [info commands :source] [:source] [namespace eval : {source}]
        }
    } result]
    interp delete r2286_s
    list $code $result
}
r2286_observe safe_global_qualification {
    interp create -safe r2286_s
    set out {}
    foreach script {{source absent.tcl} {::source absent.tcl} {::::source absent.tcl}} {
        set code [catch {interp eval r2286_s $script} result]
        lappend out $code $result
    }
    interp delete r2286_s
    set out
}
r2286_observe hide_distinct_token_and_colon_exposure {
    interp create r2286_s
    set code [catch {
        interp eval r2286_s {proc held {} {return ORIGINAL}}
        interp hide r2286_s ::held token
        set hidden [interp hidden r2286_s]
        set miss [catch {interp eval r2286_s {held}} missing]
        interp expose r2286_s token :visible
        list $hidden $miss $missing [interp eval r2286_s {:visible}] [interp eval r2286_s {info commands token}]
    } result]
    interp delete r2286_s
    list $code $result
}
r2286_observe hide_default_qualified_token {
    interp create r2286_s
    interp eval r2286_s {proc held {} {return ORIGINAL}}
    set code [catch {interp hide r2286_s ::held} result]
    set visible [interp eval r2286_s {held}]
    interp delete r2286_s
    list $code $result $visible
}
r2286_observe hide_namespace_source {
    interp create r2286_s
    interp eval r2286_s {namespace eval n {proc held {} {return NAMESPACED}}}
    set code [catch {interp hide r2286_s ::n::held token} result]
    set visible [interp eval r2286_s {n::held}]
    interp delete r2286_s
    list $code $result $visible
}
r2286_observe hidden_and_new_visible_allocations {
    interp create r2286_s
    interp eval r2286_s {proc held {} {return ORIGINAL}}
    interp hide r2286_s held token
    interp eval r2286_s {proc held {} {return REDEFINED}}
    set hidecode [catch {interp hide r2286_s held token} hidemessage]
    set exposecode [catch {interp expose r2286_s token held} exposemessage]
    set visible [interp eval r2286_s {held}]
    set original [interp invokehidden r2286_s token]
    interp delete r2286_s
    list $hidecode $hidemessage $exposecode $exposemessage $visible $original
}
r2286_observe hide_literal_colon {
    interp create r2286_s
    interp eval r2286_s {proc :held {} {return LITERAL_COLON}; proc held {} {return PLAIN}}
    interp hide r2286_s :held token
    set coloncode [catch {interp eval r2286_s {:held}} colonmessage]
    set visible [interp eval r2286_s {held}]
    set original [interp invokehidden r2286_s token]
    interp delete r2286_s
    list $coloncode $colonmessage $visible $original
}
r2286_observe counted_binary_command_boundary {
    interp create r2286_s
    set name [binary format c* {104 101 108 100 0 120}]
    interp eval r2286_s [list proc $name {} {return COUNTED}]
    set hidecode [catch {interp hide r2286_s $name token} hidemessage]
    set original [catch {interp invokehidden r2286_s token} originalmessage]
    interp delete r2286_s
    list $hidecode $hidemessage $original $originalmessage
}
rename r2286_observe {}
