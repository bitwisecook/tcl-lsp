# Jim stackdump and the -errorinfo list form
proc a {} {b}
proc b {} {error boom}
catch {a} msg opts
puts $msg
puts [lsort [dict keys $opts]]
puts [llength [stackdump [dict get $opts -errorinfo]]]
