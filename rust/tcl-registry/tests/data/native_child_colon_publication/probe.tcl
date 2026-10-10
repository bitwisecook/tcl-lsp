puts [list version [info patchlevel]]
proc r2286_observe {label script} {
    set status [catch {uplevel #0 $script} result]
    if {$status} {set code $::errorCode} else {set code {}}
    puts [list observation $label status $status result $result errorCode $code]
}
r2286_observe create {interp create -safe s}
r2286_observe literal-colon-procedure {interp eval s {proc :source {} {return literal}}}
r2286_observe literal-colon-call {interp eval s {:source}}
r2286_observe root-source-after-colon-procedure {interp eval s {source a.tcl}}
r2286_observe root-procedure-inventory {interp eval s {list [info procs :source] [info procs source]}}
r2286_observe colon-holder-procedure {interp eval s {namespace eval :ns {proc source {} {return local}}}}
r2286_observe colon-holder-procedure-inventory {interp eval s {namespace eval :ns {info procs source}}}
r2286_observe colon-holder-call {interp eval s {namespace eval :ns {source a.tcl}}}
r2286_observe uncolon-holder-procedure-inventory {interp eval s {namespace eval ns {info procs source}}}
r2286_observe uncolon-holder-call {interp eval s {namespace eval ns {source}}}
r2286_observe literal-lambda-colon-holder {interp eval s {namespace eval :ns {apply {{} {source a.tcl} :ns}}}}
r2286_observe literal-lambda-default-root {interp eval s {namespace eval :ns {apply {{} {source b.tcl}}}}}
r2286_observe delete {interp delete s}
