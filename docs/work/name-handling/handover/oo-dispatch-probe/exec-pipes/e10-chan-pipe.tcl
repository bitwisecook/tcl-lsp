# chan pipe (8.6+) wired to exec with >@ and <@.
lassign [chan pipe] r w
exec echo through-pipe >@ $w
close $w
puts "got=[gets $r]"
close $r
lassign [chan pipe] r w
puts $w "feed"
close $w
puts "fed=[exec cat <@ $r]"
close $r
