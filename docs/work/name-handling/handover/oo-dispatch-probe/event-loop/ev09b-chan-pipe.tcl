# chan event on a chan pipe (8.6+): the handler runs at level 1 from the event loop.
lassign [chan pipe] r w
chan configure $r -blocking 0
proc onread {ch} { set ::got [list [gets $ch] [info level]]; chan event $ch readable {} }
chan event $r readable [list onread $r]
puts [chan event $r readable]
puts $w hello; flush $w
after 2000 {set ::got timeout}
vwait got
puts $got
