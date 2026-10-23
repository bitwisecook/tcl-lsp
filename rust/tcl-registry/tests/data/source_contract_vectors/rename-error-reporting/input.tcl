puts [list IDENTITY [info patchlevel]]
proc hex value {binary scan $value H* result; return $result}
proc observe {label script} {
    unset -nocomplain ::errorCode
    set code [catch {uplevel 1 $script} value]
    set errorcode {}
    if {$code && [info exists ::errorCode]} {set errorcode $::errorCode}
    puts [list OBSERVATION $label $code [hex $value] [hex $errorcode]]
}
set missing [binary format H* 3a3a6d697373696e6700737566666978]
set occupied [binary format H* 3a3a6f6363757069656400737566666978]
proc ::source {} {return SOURCE}
proc $occupied {} {return OCCUPIED}
observe missing-rename {rename $missing ::destination}
observe occupied-rename {rename ::source $occupied}
observe missing-delete {rename $missing {}}
observe source-after-refusals {::source}
observe occupied-after-refusals {uplevel 0 [list $occupied]}
observe counted-missing-name {set missing}
