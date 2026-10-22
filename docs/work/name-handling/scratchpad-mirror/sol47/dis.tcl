foreach s {
  {string range $s 1 2}
  {string range $s 0 end}
  {string range $s $a $b}
  {string map {a b} $s}
  {string map $m $s}
  {string map -nocase {a b} $s}
  {string replace $s 1 2}
  {string replace $s 1 2 x}
  {string is integer $s}
  {string is integer -strict $s}
  {string cat a $b c}
  {string insert $s 1 x}
} {
  puts "=== $s"
  if {[catch {puts [::tcl::unsupported::disassemble script $s]} e]} {puts "ERR $e"}
}
