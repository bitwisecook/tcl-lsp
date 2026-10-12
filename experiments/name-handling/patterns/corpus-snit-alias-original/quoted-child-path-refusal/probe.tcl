# Fixed authored caller composition over unmodified Tcllib interp/Snit packages.
proc probe_field {name value} {
    binary scan $value H* encoded
    puts "FIELD $name $encoded"
}
set corpus [lindex $argv 0]
set setup_code [catch {
    lappend auto_path [file join $corpus modules interp]
    lappend auto_path [file join $corpus modules snit]
    set interp_package [package require interp 0.1.3]
    set snit_package [package require snit]
    snit::type ::AliasProbe {
        method install {child} {
            set prefix [mymethod {echo space} {captured arg}]
            ::interp::snitLink $child [list [list {echo space} {captured arg}]]
            return $prefix
        }
        method change {child} {
            set prefix [mymethod {missing method} {captured arg}]
            ::interp::snitDictLink $child [list {echo space} [list {missing method} {captured arg}]]
            return $prefix
        }
        method {echo space} {captured value} {
            return [list fixed $captured $value]
        }
    }
    ::AliasProbe {::object space}
    set child [interp create [list {child space}]]
} setup_result]
probe_field setup_code $setup_code
probe_field setup_result $setup_result
if {$setup_code} {
    if {[info exists ::errorCode]} {probe_field setup_error_code $::errorCode}
    probe_field positive_code not-run
    probe_field changed_code not-run
    exit 0
}
probe_field interp_package $interp_package
probe_field snit_package $snit_package
probe_field child_identity $child
set install_code [catch {{::object space} install $child} original_prefix]
probe_field install_code $install_code
probe_field original_prefix $original_prefix
if {$install_code} {
    probe_field positive_code not-run
    probe_field changed_code not-run
    exit 0
}
set stored_prefix [interp alias $child {echo space}]
probe_field stored_prefix $stored_prefix
probe_field prefix_equal [string equal $stored_prefix $original_prefix]
set positive_code [catch {interp eval $child [list {echo space} {arg tail}]} positive_result]
probe_field positive_code $positive_code
probe_field positive_result $positive_result
set change_code [catch {{::object space} change $child} changed_prefix]
probe_field change_code $change_code
probe_field changed_prefix $changed_prefix
if {$change_code} {
    probe_field changed_code not-run
    exit 0
}
set changed_stored_prefix [interp alias $child {echo space}]
probe_field changed_stored_prefix $changed_stored_prefix
probe_field changed_prefix_equal [string equal $changed_stored_prefix $changed_prefix]
probe_field prefix_changed [expr {![string equal $stored_prefix $changed_stored_prefix]}]
set changed_code [catch {interp eval $child [list {echo space} {arg tail}]} changed_result]
probe_field changed_code $changed_code
probe_field changed_result $changed_result
if {$changed_code && [info exists ::errorCode]} {probe_field changed_error_code $::errorCode}
interp delete $child
{::object space} destroy
