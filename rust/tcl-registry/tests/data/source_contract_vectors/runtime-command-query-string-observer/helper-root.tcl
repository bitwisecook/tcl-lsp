puts [list IDENTITY [info patchlevel]]
proc ::__no_root_observe {label script} {
    unset -nocomplain ::errorCode ::errorInfo
    set code [catch {uplevel 1 $script} value]
    set errorcode {}
    switch -exact $code 0 {} default {switch -exact [info exists ::errorCode] 1 {set errorcode $::errorCode}}
    puts [list OBSERVATION $label $code $value $errorcode]
}
proc ::__no_root_first {} {return FIRST}
proc ::__no_root_second {} {return SECOND}
set ::__no_root_jim [llength [info commands alias]]
proc ::__no_root_alias {name target} {
    switch -exact $::__no_root_jim 0 {interp alias {} $name {} $target} default {alias $name $target}
    return {}
}
namespace eval ::__no_root_basis {
    namespace eval __no_root_basis {proc target {} {return DOUBLED}}
    proc target {} {return LOCAL}
    ::__no_root_observe first-alias-setup {::__no_root_alias ::__no_root_basis::choice ::__no_root_first}
    ::__no_root_observe doubled-alias-setup {::__no_root_alias ::__no_root_basis::__no_root_basis::choice ::__no_root_second}
    ::__no_root_observe helper-root-alias-origin {namespace origin choice}
    ::__no_root_observe helper-root-command-origin {namespace origin target}
    ::__no_root_observe helper-root-which {namespace which -command choice}
}
