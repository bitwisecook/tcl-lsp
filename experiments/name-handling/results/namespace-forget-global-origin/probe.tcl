# Original issue #2300 setup/operation, followed by fixed adverse controls.
proc field {name value} {
    binary scan $value H* encoded
    puts "FIELD $name $encoded"
}
set setup_code [catch {
    proc g {} {}
    namespace eval :: {namespace export g}
    namespace eval n {namespace import ::g}
} setup_result]
field setup_code $setup_code
field setup_result $setup_result
if {$setup_code} {
    field positive_code not-run
    field adverse_code not-run
    exit 0
}
set positive_code [catch {
    namespace eval n {namespace forget ::g}
    info commands n::g
} positive_result]
field positive_code $positive_code
field positive_result $positive_result
set adverse_setup_code [catch {
    namespace eval other {proc h {} {return OTHER}; namespace export h}
    namespace eval n {namespace import ::other::h; proc local {} {return LOCAL}}
} adverse_setup_result]
field adverse_setup_code $adverse_setup_code
field adverse_setup_result $adverse_setup_result
if {$adverse_setup_code} {
    field adverse_code not-run
    exit 0
}
set adverse_code [catch {namespace eval n {namespace forget ::missing::g}} adverse_result]
field adverse_code $adverse_code
field adverse_result $adverse_result
if {$adverse_code && [info exists ::errorCode]} {field adverse_error_code $::errorCode}
field imported_other_present [llength [info commands ::n::h]]
field local_forget_code [catch {namespace eval n {namespace forget ::n::*}} local_forget_result]
field local_forget_result $local_forget_result
field local_present [llength [info commands ::n::local]]
field imported_other_after_local_forget [llength [info commands ::n::h]]
field global_source_present [llength [info commands ::g]]
