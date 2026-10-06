set count 0
proc relay args {incr ::count; return $args}
if {[catch {interp create} child]} {
    set child [interp]
    set handle_api 1
    $child alias forward relay PREFIX
} else {
    set handle_api 0
    interp alias $child forward {} relay PREFIX
}
puts [$child eval {forward X}]
rename relay saved
proc relay args {return [linsert $args 0 NEW]}
puts [$child eval {forward X}]
puts $count
if {$handle_api} {$child delete} else {interp delete $child}
