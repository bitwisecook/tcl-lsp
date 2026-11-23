# > file, >> file, 2> file, >& file, 2>@1, >@ channel, 2>@ channel
exec echo one > out.txt
exec echo two >> out.txt
puts "gt-append=[exec cat out.txt]"
catch {exec cat no-such-file 2> err.txt} m
puts "two-gt=[string match *no-such-file* [exec cat err.txt]] result=<[string match *no-such-file* $m]>"
catch {exec cat no-such-file >& both.txt}
puts "gt-amp=[string match *no-such-file* [exec cat both.txt]]"
puts "two-at-one=[catch {exec cat no-such-file 2>@1} m] [string match *no-such-file* $m]"
set ch [open chan.txt w]
exec echo to-chan >@ $ch
close $ch
puts "gt-at=[exec cat chan.txt]"
file delete out.txt err.txt both.txt chan.txt
