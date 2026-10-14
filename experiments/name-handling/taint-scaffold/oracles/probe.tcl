set script {::proc ordinary args {}
::proc interp args {}
::proc z args {}
::proc proc args {}
}
set code [catch {eval $script} result]
puts [list $code $result [lsort [info procs ordinary]] [lsort [info procs interp]] [lsort [info procs z]] [lsort [info procs proc]]]
