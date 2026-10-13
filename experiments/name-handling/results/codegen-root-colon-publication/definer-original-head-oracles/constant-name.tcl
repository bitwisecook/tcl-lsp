set name :audit
proc $name {} {return ORIGINAL}
set answer [:audit]
puts [list $answer [info commands audit]]
