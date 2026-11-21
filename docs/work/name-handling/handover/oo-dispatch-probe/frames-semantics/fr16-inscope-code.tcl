namespace eval ::n { proc lv {} { info level } }
proc p {} { list [namespace inscope ::n lv] [eval [namespace eval ::n {namespace code lv}]] [namespace eval ::n {info level}] [namespace eval ::n {lv}] }
puts [p]
