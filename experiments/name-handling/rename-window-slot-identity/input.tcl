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
        sibling-replace {
            namespace eval ::__nh_collision: {
                ::__nh_measure [list $::__nh_scenario B-delete] [list rename $::__nh_name_b {}]
                proc p {} {return [list B-new [namespace current]]}
                ::__nh_measure [list $::__nh_scenario B-replacement-call] [list $::__nh_name_b]
                ::__nh_measure [list $::__nh_scenario B-replacement-traces] [list trace info command $::__nh_name_b]
            }
            ::__nh_measure [list $::__nh_scenario A-destination-call] [list ::__nh_dst::m]
        }
        destination-replace {
            ::__nh_measure [list $::__nh_scenario A-destination-delete] [list rename ::__nh_dst::m {}]
            proc ::__nh_dst::m {} {return [list A-new [namespace current]]}
            namespace eval ::__nh_collision {
                ::__nh_measure [list $::__nh_scenario A-source-during-call] [list $::__nh_name_a]
                ::__nh_measure [list $::__nh_scenario A-source-during-traces] [list trace info command $::__nh_name_a]
            }
            namespace eval ::__nh_collision: {
                ::__nh_measure [list $::__nh_scenario B-current-call] [list $::__nh_name_b]
                ::__nh_measure [list $::__nh_scenario B-current-traces] [list trace info command $::__nh_name_b]
            }
            ::__nh_measure [list $::__nh_scenario A-replacement-call] [list ::__nh_dst::m]
            ::__nh_measure [list $::__nh_scenario A-replacement-traces] [list trace info command ::__nh_dst::m]
        }
        retained-namespace {
            namespace eval ::__nh_collision: {
                ::__nh_measure [list $::__nh_scenario B-retained-before-call] [list $::__nh_name_b]
                ::__nh_measure [list $::__nh_scenario B-namespace-delete] [list namespace delete ::__nh_collision:]
                namespace eval ::__nh_collision: {proc p {} {return [list B-new [namespace current]]}}
                ::__nh_measure [list $::__nh_scenario B-retained-current] [list namespace current]
                ::__nh_measure [list $::__nh_scenario B-retained-after-call] [list $::__nh_name_b]
                ::__nh_measure [list $::__nh_scenario B-retained-after-traces] [list trace info command $::__nh_name_b]
                ::__nh_measure [list $::__nh_scenario B-visible-new-call] [list namespace eval ::__nh_collision: [list $::__nh_name_b]]
                ::__nh_measure [list $::__nh_scenario B-visible-new-traces] [list namespace eval ::__nh_collision: [list trace info command $::__nh_name_b]]
            }
            ::__nh_measure [list $::__nh_scenario A-destination-call] [list ::__nh_dst::m]
        }
    }
    puts [list [list $::__nh_scenario trace-A-exit] 0 {} {}]
}
set scenarios $argv
if {[llength $scenarios] == 0} {set scenarios {info nested sibling-replace destination-replace retained-namespace}}
foreach scenario $scenarios {
    set ::__nh_scenario cleanup
    foreach name {::__nh_collision ::__nh_collision: ::__nh_dst} {catch {namespace delete $name}}
    set ::__nh_scenario $scenario
    set setup [catch {
        namespace eval ::__nh_collision {proc :p {} {return [list A [namespace current]]}}
        namespace eval ::__nh_collision: {proc p {} {return [list B [namespace current]]}}
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
        ::__nh_measure [list $::__nh_scenario B-trace-install] [list trace add command $::__nh_name_b [list rename delete] ::__nh_trace_b]
    }]
    if {$trace_a || $trace_b} {
        puts [list [list $scenario unsupported-command-trace] 1 {window body not reached; initial physical holders retained above} {}]
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
