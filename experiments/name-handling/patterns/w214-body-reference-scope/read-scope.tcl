proc plain {parameter} {namespace eval NS {puts $parameter}}
proc bom {parameter} {﻿namespace eval NS {puts $parameter}}
proc direct {parameter} {puts $parameter}
proc inert {parameter} {set text {$parameter}; return inert}
foreach name {plain bom direct inert} {
    set code [catch {$name ORIGINAL} message]
    puts [list $name $code $message]
}
proc sugar {parameter} {return $($parameter + 1)}
proc nestedSugar {parameter} {return $([list $parameter] + 1)}
foreach name {sugar nestedSugar} {
    set code [catch {$name 5} message]
    puts [list $name $code $message]
}
