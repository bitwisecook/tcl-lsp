puts [list IDENTITY [info patchlevel]]
proc ::__jfn_observe {label script} {
    unset -nocomplain ::errorCode ::errorInfo
    set code [catch {uplevel 1 $script} value]
    set errorcode {}
    if {$code && [info exists ::errorCode]} {set errorcode $::errorCode}
    puts [list OBSERVATION $label $code $value $errorcode]
}
proc ::__jfn_target {} {return TARGET}
set old [format "::alias%cold" 0]
set new [format "::alias%cnew" 0]
::__jfn_observe original-old {list [string length $old] $old}
::__jfn_observe original-new {list [string length $new] $new}
::__jfn_observe old-create {alias $old ::__jfn_target}
::__jfn_observe old-query {info alias $old}
::__jfn_observe old-direct {uplevel 0 [list $old]}
::__jfn_observe old-rename {rename $old ::moved}
::__jfn_observe moved-query {info alias ::moved}
::__jfn_observe old-query-after-rename {info alias $old}
::__jfn_observe new-create {alias $new ::__jfn_target}
::__jfn_observe new-query {info alias $new}
::__jfn_observe moved-direct-after {::moved}
