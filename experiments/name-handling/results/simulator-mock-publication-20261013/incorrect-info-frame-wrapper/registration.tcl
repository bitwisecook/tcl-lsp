namespace eval ::legacy {
    proc _mock_proc_name {irule_cmd} {
        if {[string first "::" $irule_cmd] >= 0} {
            set parts [split $irule_cmd "::"]
            # split on :: gives {NS {} sub}
            set ns [string tolower [lindex $parts 0]]
            set sub [lindex $parts end]
            set ns [string map {- _ . _} $ns]
            set sub [string map {- _ . _} $sub]
            return "::itest::cmd::${ns}_${sub}"
        } else {
            set safe [string map {- _ . _} $irule_cmd]
            return "::itest::cmd::cmd_${safe}"
        }
    }

}
namespace eval ::tmm {}
proc ::tmm::_orig_info {args} {uplevel #0 [linsert $args 0 info]}
source [file join [file dirname [info script]] fixture-mock-names.tcl]
namespace eval ::itest::cmd {
    proc _mock_proc_name {irule_cmd} {
        variable _mock_names
        if {[::tmm::_orig_info exists _mock_names($irule_cmd)]} {
            return $_mock_names($irule_cmd)
        }
        return ""
    }

}
puts [list PROVIDER [info patchlevel]]
foreach label {HTTP::header ::NS::inner::cmd NS::inner::cmd NS::cmd traffic-group a:b :HTTP::cmd HTTP:::header ::::HTTP::header é __unlisted__} {
 set old [::legacy::_mock_proc_name $label]
 set new [::itest::cmd::_mock_proc_name $label]
 set reached {}
 if {$new ne ""} {proc $new {} {return OK}; set reached [namespace which -command $new]}
 puts [list CASE $label $old $new $reached]
}
