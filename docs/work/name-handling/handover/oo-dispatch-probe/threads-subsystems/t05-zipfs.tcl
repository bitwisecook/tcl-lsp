# zipfs (9.0): build an archive, mount it, read through //zipfs:/.
if {[catch {zipfs root} root]} { puts "no-zipfs: $root"; return }
puts "root=$root"
file mkdir src
set f [open src/hello.txt w]; puts $f "zipped text"; close $f
zipfs mkzip demo.zip src src
zipfs mount demo.zip /demo
set mp [file join $root demo]
puts "mounted=[expr {[lsearch -exact [dict keys [zipfs mount]] $mp] >= 0}]"
puts "list=[lsort [zipfs list -glob $mp/*]]"
set ch [open $mp/hello.txt r]; puts "read=[gets $ch]"; close $ch
puts "exists=[file exists $mp/hello.txt]"
puts "info=[file tail [lindex [zipfs info $mp/hello.txt] 0]]"
puts "canon=[zipfs canonical demo x.txt]"
zipfs unmount /demo
puts "after=[file exists $mp/hello.txt]"
file delete -force src demo.zip
