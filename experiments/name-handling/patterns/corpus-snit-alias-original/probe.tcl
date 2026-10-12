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
            set prefix [mymethod echo {captured arg}]
            ::interp::snitLink $child [list [list echo {captured arg}]]
            return $prefix
        }
        method change {child} {
            set prefix [mymethod {missing method} {captured arg}]
            ::interp::snitDictLink $child [list echo [list {missing method} {captured arg}]]
            return $prefix
        }
        method echo {captured value} {
            return [list fixed $captured $value]
        }
    }
    ::AliasProbe ::aliasObject
    set child [interp create aliasChild]
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
set install_code [catch {::aliasObject install $child} original_prefix]
probe_field install_code $install_code
probe_field original_prefix $original_prefix
if {$install_code} {
    probe_field positive_code not-run
    probe_field changed_code not-run
    exit 0
}
set stored_prefix [interp alias $child echo]
probe_field stored_prefix $stored_prefix
probe_field prefix_equal [string equal $stored_prefix $original_prefix]
set positive_code [catch {interp eval $child [list echo {arg tail}]} positive_result]
probe_field positive_code $positive_code
probe_field positive_result $positive_result
set change_code [catch {::aliasObject change $child} changed_prefix]
probe_field change_code $change_code
probe_field changed_prefix $changed_prefix
if {$change_code} {
    probe_field changed_code not-run
    exit 0
}
set changed_stored_prefix [interp alias $child echo]
probe_field changed_stored_prefix $changed_stored_prefix
probe_field changed_prefix_equal [string equal $changed_stored_prefix $changed_prefix]
probe_field prefix_changed [expr {![string equal $stored_prefix $changed_stored_prefix]}]
set changed_code [catch {interp eval $child [list echo {arg tail}]} changed_result]
probe_field changed_code $changed_code
probe_field changed_result $changed_result
if {$changed_code && [info exists ::errorCode]} {probe_field changed_error_code $::errorCode}
probe_field child_cleanup_code [catch {interp delete $child} child_cleanup_result]
probe_field child_cleanup_result $child_cleanup_result
probe_field object_cleanup_code [catch {::aliasObject destroy} object_cleanup_result]
probe_field object_cleanup_result $object_cleanup_result
