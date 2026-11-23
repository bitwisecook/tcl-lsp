# No shell: words arrive verbatim, including spaces, ; and $.
puts "spaces=[exec echo {a  b}]"
puts "semi=[exec echo {x;y}]"
puts "dollar=[exec echo {$HOME}]"
puts "star=[exec echo {*}]"
# BUT exec scans each word for redirection/pipe syntax: a substituted
# value starting with < > | 2> & changes the command (injection hazard).
set user {<user.txt}
set f [open user.txt w]; puts $f "leaked contents"; close $f
puts "inject-lt=[exec echo $user]"
set user {>made.txt}
exec echo harmless $user
puts "inject-gt=[file exists made.txt] [exec cat made.txt]"
set user {|}
puts "inject-bar=[catch {exec echo a $user cat} m] $m"
# -- only ends exec's own switches; it does not protect later words.
set user {<user.txt}
puts "dashdash=[exec -- echo $user]"
file delete user.txt made.txt
