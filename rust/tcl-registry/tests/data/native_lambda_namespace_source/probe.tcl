puts [list version [info patchlevel]]
proc r2286_observe {label script} {
    set status [catch {uplevel #0 $script} result]
    if {$status} {set code $::errorCode} else {set code {}}
    puts [list observation $label status $status result $result errorCode $code]
}
r2286_observe ns-create {namespace eval ns {}}
r2286_observe colon-ns-create {namespace eval :ns {}}
r2286_observe relative-colon {apply [list {} {namespace current} :ns]}
r2286_observe absolute-colon {apply [list {} {namespace current} :::ns]}
r2286_observe relative-plain {apply [list {} {namespace current} ns]}
r2286_observe absolute-plain {apply [list {} {namespace current} ::ns]}
r2286_observe default-root {apply {{} {namespace current}}}
r2286_observe empty-root {apply [list {} {namespace current} {}]}
r2286_observe counted-relative-colon {set name [binary format c* {58 110 115 0 115 117 102 102 105 120}]; apply [list {} {namespace current} $name]}
r2286_observe counted-absolute {set name [binary format c* {58 58 110 115 0 115 117 102 102 105 120}]; apply [list {} {namespace current} $name]}
r2286_observe counted-leading-null {set name [binary format c* {0 115 117 102 102 105 120}]; apply [list {} {namespace current} $name]}
r2286_observe unicode-ns-create {namespace eval é {}; namespace eval :é {} }
r2286_observe unicode-relative-colon {apply [list {} {namespace current} :é]}
