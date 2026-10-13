puts [list PATCHLEVEL [info patchlevel]]
set ::__nh_name_a :p
set ::__nh_name_b p
proc ::__nh_measure {id script} {
    unset -nocomplain ::errorCode ::errorInfo
    set code [catch {uplevel 1 $script} value]
    set errorcode {}
    if {$code && [info exists ::errorCode]} {set errorcode $::errorCode}
    puts [list $id $code $value $errorcode]
    return $code
}
proc ::__nh_trace_extra {old new operation} {}
proc ::__nh_trace_b {old new operation} {
    puts [list [list $::__nh_scenario trace-B] 0 [list $old $new $operation] {}]
}
proc ::__nh_trace_a {old new operation} {
    puts [list [list $::__nh_scenario trace-A-enter] 0 [list $old $new $operation] {}]
    switch -- $::__nh_scenario {
        info {
            namespace eval ::__nh_collision: {
                ::__nh_measure [list $::__nh_scenario B-current-call] [list $::__nh_name_b]
                ::__nh_measure [list $::__nh_scenario B-current-traces] [list trace info command $::__nh_name_b]
                ::__nh_measure [list $::__nh_scenario B-current-which] [list namespace which -command $::__nh_name_b]
                ::__nh_measure [list $::__nh_scenario B-add-trace] [list trace add command $::__nh_name_b rename ::__nh_trace_extra]
                ::__nh_measure [list $::__nh_scenario B-added-traces] [list trace info command $::__nh_name_b]
                ::__nh_measure [list $::__nh_scenario B-remove-trace] [list trace remove command $::__nh_name_b rename ::__nh_trace_extra]
                ::__nh_measure [list $::__nh_scenario B-remaining-traces] [list trace info command $::__nh_name_b]
            }
            ::__nh_measure [list $::__nh_scenario A-destination-call] [list ::__nh_dst::m]
            ::__nh_measure [list $::__nh_scenario A-destination-traces] [list trace info command ::__nh_dst::m]
        }
        nested {
            namespace eval ::__nh_collision: {
                ::__nh_measure [list $::__nh_scenario B-nested-rename] [list rename $::__nh_name_b ::__nh_dst::b]
                ::__nh_measure [list $::__nh_scenario B-vacated-call] [list $::__nh_name_b]
            }
            ::__nh_measure [list $::__nh_scenario B-new-call] [list ::__nh_dst::b]
            ::__nh_measure [list $::__nh_scenario A-destination-call] [list ::__nh_dst::m]
        }

    }
    puts [list [list $::__nh_scenario trace-A-exit] 0 {} {}]
}
set scenarios $argv
if {[llength $scenarios] == 0} {set scenarios {info}}
if {[llength $scenarios] != 1} {error "select one ordinary scenario per fresh interpreter: info or nested"}
foreach scenario $scenarios {if {[lsearch -exact {info nested} $scenario] < 0} {error "unavailable ordinary scenario: $scenario"}}
foreach scenario $scenarios {
    set ::__nh_scenario $scenario
    set setup [catch {
        namespace eval ::__nh_collision {
            proc original_a {} {return [list A [namespace current]]}
            rename original_a $::__nh_name_a
        }
        proc ::__nh_original_b {} {return [list B [namespace current]]}
        namespace eval ::__nh_collision: {rename ::__nh_original_b $::__nh_name_b}
        namespace eval ::__nh_dst {}
    } value]
    puts [list [list $scenario setup] $setup $value {}]
    if {$setup} {continue}
    namespace eval ::__nh_collision {
        ::__nh_measure [list $::__nh_scenario A-before-call] [list $::__nh_name_a]
        ::__nh_measure [list $::__nh_scenario A-before-which] [list namespace which -command $::__nh_name_a]
    }
    namespace eval ::__nh_collision: {
        ::__nh_measure [list $::__nh_scenario B-before-call] [list $::__nh_name_b]
        ::__nh_measure [list $::__nh_scenario B-before-which] [list namespace which -command $::__nh_name_b]
    }
    set trace_a [namespace eval ::__nh_collision {
        ::__nh_measure [list $::__nh_scenario A-trace-install] [list trace add command $::__nh_name_a rename ::__nh_trace_a]
    }]
    set trace_b [namespace eval ::__nh_collision: {
        ::__nh_measure [list $::__nh_scenario B-trace-install] [list trace add command $::__nh_name_b rename ::__nh_trace_b]
    }]
    if {$trace_a || $trace_b} {
        puts [list [list $scenario unavailable-window-setup] 1 {window body not reached; prefix observations retained above} {}]
        continue
    }
    namespace eval ::__nh_collision {
        ::__nh_measure [list $::__nh_scenario A-outer-rename] [list rename $::__nh_name_a ::__nh_dst::m]
        ::__nh_measure [list $::__nh_scenario A-after-source-call] [list $::__nh_name_a]
    }
    namespace eval ::__nh_collision: {
        ::__nh_measure [list $::__nh_scenario B-after-call] [list $::__nh_name_b]
        ::__nh_measure [list $::__nh_scenario B-after-traces] [list trace info command $::__nh_name_b]
    }
    ::__nh_measure [list $scenario A-after-destination-call] [list ::__nh_dst::m]
    ::__nh_measure [list $scenario B-after-new-call] [list ::__nh_dst::b]
}
set ::__nh_scenario cleanup
