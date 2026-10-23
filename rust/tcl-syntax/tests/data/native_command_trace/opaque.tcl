proc hex {v} {binary scan $v H* h; return $h}
set log {}
proc cbÿ args {lappend ::log [lindex $args end]}
proc pÿ {} {return OK}
trace add command pÿ {rename delete} cbÿ
trace add execution pÿ {enter leave} cbÿ
puts [hex [lindex [lindex [trace info command pÿ] 0] 1]]
puts [hex [lindex [lindex [trace info execution pÿ] 0] 1]]
pÿ
trace remove execution pÿ {enter leave} cbÿ
puts [llength [trace info execution pÿ]]
rename pÿ qÿ
puts [llength [trace info command qÿ]]
trace remove command qÿ {rename delete} cbÿ
puts [llength [trace info command qÿ]]
trace add command qÿ delete cbÿ
rename qÿ {}
puts $log
