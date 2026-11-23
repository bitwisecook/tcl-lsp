# An idle callback registered by an idle callback: does one update idletasks run it?
after idle {lappend ::o a; after idle {lappend ::o b}}
update idletasks
puts $o
update idletasks
puts $o
