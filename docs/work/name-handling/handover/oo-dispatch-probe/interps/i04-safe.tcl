# A safe interpreter hides unsafe commands; they remain invokable by the parent.
set s [interp create -safe]
puts "issafe=[interp issafe $s]"
set hidden [lsort [interp hidden $s]]
foreach cmd {exec open source file cd exit load socket glob pwd} {
  puts "$cmd hidden=[expr {[lsearch -exact $hidden $cmd] >= 0}]"
}
puts "exec-in-safe=[catch {interp eval $s {exec echo hi}} m] $m"
# hidden pwd still works for the parent
puts "same=[string equal [interp invokehidden $s pwd] [pwd]]"
interp delete $s
