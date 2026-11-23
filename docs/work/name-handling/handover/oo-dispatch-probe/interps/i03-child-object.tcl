# The child's name is also a command in the parent: $c eval, $c alias, ...
proc hello {who} { return "hello $who" }
set c [interp create kid]
puts "cmd=[llength [info commands kid]]"
puts "eval=[$c eval {string toupper abc}]"
$c alias greet hello
puts "alias=[$c eval greet world]"
puts "issafe=[$c issafe]"
puts "aliases=[$c aliases]"
kid eval {set y 9}
puts "y=[kid eval {set y}]"
interp delete kid
puts "cmd-after=[llength [info commands kid]]"
