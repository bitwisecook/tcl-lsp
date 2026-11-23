# default channel options on a new file channel
set d [file join [pwd] t06]
set f [open $d w]
foreach o {-encoding -translation -eofchar -buffering -blocking} {
    puts "w $o: [list [fconfigure $f $o]]"
}
puts "w profile: [catch {fconfigure $f -profile} m] $m"
close $f
set f [open $d r]
foreach o {-encoding -translation -eofchar} {
    puts "r $o: [list [fconfigure $f $o]]"
}
fconfigure $f -translation binary
puts "after binary: [fconfigure $f -encoding] [fconfigure $f -translation] [list [fconfigure $f -eofchar]]"
close $f
puts "stdout buffering: [fconfigure stdout -buffering]"
puts "chan configure same: [expr {[info commands chan] ne {}}]"
file delete $d
