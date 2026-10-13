set ::fixture [lindex $argv 0]
puts [list PROVIDER [info patchlevel]]
proc emit {id types} {
    set ::errorCode {NH UNSET}
    set command [list glob -nocomplain -tails -directory $::fixture -types $types -- *]
    set code [catch $command result]
    if {$code == 0} {set result [lsort $result]}
    puts [list ROW $id $code $result $::errorCode]
}
emit readable-writable-executable {r w x}
