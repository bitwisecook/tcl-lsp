puts [list IDENTITY [info patchlevel]]
proc ::__sr_observe {label script} {
    unset -nocomplain ::errorCode ::errorInfo
    set code [catch {uplevel 1 $script} value]
    set errorcode {}
    if {$code && [info exists ::errorCode]} {set errorcode $::errorCode}
    puts [list OBSERVATION $label $code $value $errorcode]
}
::__sr_observe private-presence {info commands ::tcl::namespace::which}
::__sr_observe configured-holder {namespace ensemble configure namespace -namespace}
::__sr_observe private-original {::tcl::namespace::which -command list}
::__sr_observe relative-map {namespace ensemble configure namespace -map {which which}}
::__sr_observe relative-map-other-caller {namespace eval ::caller {namespace which -command list}}
::__sr_observe original-relocation {rename ::tcl::namespace::which ::retainedWorker}
::__sr_observe retained-direct {::retainedWorker -command list}
::__sr_observe original-slot-replacement {proc ::tcl::namespace::which {args} {return REPLACEMENT}}
::__sr_observe public-after-replacement {namespace which -command list}
::__sr_observe retained-after-replacement {::retainedWorker -command list}
