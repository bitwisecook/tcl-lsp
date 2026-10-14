puts [list provider [info patchlevel]]
set arr(KEY) FIRST
set arr(OTHER) SECOND
set cmd KEY
set first "$arr($cmd)"
set cmd OTHER
set second "$arr($cmd)"
puts [list indexed_values $first $second]
unset cmd
set code [catch {set value "$arr($cmd)"} result]
puts [list missing_index $code $result]
set cmd KEY
unset arr
set code [catch {set value "$arr($cmd)"} result]
puts [list missing_array $code $result]
