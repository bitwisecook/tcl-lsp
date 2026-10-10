# Original ASCII source-file property layout controls. These observations concern
# stock Tcl commands, stored method scripts and configure calls, not Native
# source-carrier admission, method headers, compiler activation or BIG-IP.
puts [list META patchlevel [info patchlevel] configurable_command [llength [info commands ::oo::configurable]]]
if {![llength [info commands ::oo::configurable]]} {
    puts [list UNAVAILABLE reason no_original_oo_configurable_command]
} else {
    proc ::-set {} {return getter_option_value}
    ::oo::configurable create ::property_width_p {
        property p -get {-set} "-set" {set ::property_width_seen $value}
    }
    foreach method {<ReadProp-p> <WriteProp-p>} {
        set code [catch [list info class definition ::property_width_p $method] result]
        puts [list DEFINITION label option_like_getter method $method code $code definition $result]
    }
    set instance [::property_width_p new]
    set code [catch {$instance configure -p} result]
    puts [list CALL label option_like_getter operation read code $code result $result]
    set ::property_width_seen not_observed
    set code [catch {$instance configure -p setter_value} result]
    puts [list CALL label option_like_getter operation write code $code result $result seen $::property_width_seen]
    $instance destroy
    ::property_width_p destroy
    rename ::-set {}

    ::oo::configurable create ::property_width_multi {
        property p -g {-set} q "-s" {set ::property_width_q $value}
    }
    foreach method {<ReadProp-p> <ReadProp-q> <WriteProp-q>} {
        set code [catch [list info class definition ::property_width_multi $method] result]
        puts [list DEFINITION label multiple_names_abbreviations method $method code $code definition $result]
    }
    ::property_width_multi destroy

    ::oo::configurable create ::property_width_repeat {
        property p -get {return superseded} "-get" {-set} -set {set ::property_width_repeat_seen $value}
    }
    foreach method {<ReadProp-p> <WriteProp-p>} {
        set code [catch [list info class definition ::property_width_repeat $method] result]
        puts [list DEFINITION label repeated_getter method $method code $code definition $result]
    }
    ::property_width_repeat destroy

    foreach {label definition} {
        missing_value {property p -get}
        unknown_option {property p -unknown {return hidden}}
        ambiguous_option {property p - {return hidden}}
        invalid_kind {property p -kind r -get {return hidden}}
    } {
        set code [catch [list ::oo::configurable create ::property_width_invalid $definition] result]
        set ec {}
        if {$code != 0 && [info exists ::errorCode]} {set ec $::errorCode}
        puts [list DECL label $label code $code result $result errorCode $ec]
        if {$code == 0} {::property_width_invalid destroy}
    }
}
