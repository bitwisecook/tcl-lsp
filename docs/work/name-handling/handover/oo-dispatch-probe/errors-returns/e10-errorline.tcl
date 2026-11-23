# -errorline counts lines in the script that raised the error.
catch {
  set a 1
  set b 2
  error three
} m o
puts "line: [dict get $o -errorline]"
proc p {} {

  error inproc
}
catch {p} m o
puts "proc line: [dict get $o -errorline]"
puts "info: [lindex [split $::errorInfo \n] end]"
