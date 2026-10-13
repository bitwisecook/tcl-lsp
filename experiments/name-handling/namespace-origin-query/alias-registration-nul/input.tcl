puts [list IDENTITY [info patchlevel]]
proc ::__an_hex {value} {binary scan $value H* hex; return $hex}
proc ::__an_observe {label script} {
    unset -nocomplain ::errorCode ::errorInfo
    set code [catch {uplevel 1 $script} value]
    set errorcode {}
    if {$code && [info exists ::errorCode]} {set errorcode $::errorCode}
    puts [list OBSERVATION $label $code [::__an_hex $value] [::__an_hex $errorcode]]
}
proc ::__an_inventory {} {
    set names {}
    foreach name [interp aliases {}] {lappend names [::__an_hex $name]}
    return [lsort $names]
}
proc ::__an_target {} {return TARGET}
set old [binary format H* 3a3a616c696173006f6c64]
set new [binary format H* 3a3a616c696173006e6577]
::__an_observe original-old {set old}
::__an_observe original-new {set new}
::__an_observe initial-inventory {::__an_inventory}
::__an_observe old-create {interp alias {} $old {} ::__an_target}
::__an_observe old-query {interp alias {} $old}
::__an_observe old-direct {uplevel 0 [list $old]}
::__an_observe old-rename {rename $old ::moved}
::__an_observe moved-direct-before {::moved}
::__an_observe moved-origin-before {namespace origin ::moved}
::__an_observe old-query-after-rename {interp alias {} $old}
::__an_observe old-direct-after-rename {uplevel 0 [list $old]}
::__an_observe new-create {interp alias {} $new {} ::__an_target}
::__an_observe new-query {interp alias {} $new}
::__an_observe moved-direct-after {::moved}
::__an_observe moved-origin-after {namespace origin ::moved}
::__an_observe old-query-after-new {interp alias {} $old}
::__an_observe new-direct {uplevel 0 [list $new]}
::__an_observe final-inventory {::__an_inventory}
