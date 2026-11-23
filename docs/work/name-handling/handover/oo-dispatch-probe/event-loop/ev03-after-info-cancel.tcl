# after info, after cancel by id and by script text, and after with several script words.
set id [after 1000 {set ::never 1}]
puts [after info $id]
after cancel $id
puts [list [after info] [catch {after info $id} m] $m]
after 1000 set ::never 2
after cancel set ::never 2
puts [list [after info]]
after 0 set ::joined ok
after 10 {set ::done 1}
vwait done
puts [list $joined [info exists never]]
