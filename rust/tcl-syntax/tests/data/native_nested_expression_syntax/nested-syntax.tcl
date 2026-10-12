set x NONE
set expression "abs(\[set x {])"
set code [catch {expr $expression} value]
set hex {};foreach ch [split $value {}] {scan $ch %c number;append hex [format %02x $number]}
puts "brace-abs	$code	$hex	$x"
set x NONE
set expression "pow(\[set x {])"
set code [catch {expr $expression} value]
set hex {};foreach ch [split $value {}] {scan $ch %c number;append hex [format %02x $number]}
puts "brace-pow	$code	$hex	$x"
set x NONE
set expression "future(\[set x {])"
set code [catch {expr $expression} value]
set hex {};foreach ch [split $value {}] {scan $ch %c number;append hex [format %02x $number]}
puts "brace-unknown	$code	$hex	$x"
set x NONE
set expression "1 + \[set x {])"
set code [catch {expr $expression} value]
set hex {};foreach ch [split $value {}] {scan $ch %c number;append hex [format %02x $number]}
puts "brace-add	$code	$hex	$x"
set x NONE
set expression "abs(\[set x \"])"
set code [catch {expr $expression} value]
set hex {};foreach ch [split $value {}] {scan $ch %c number;append hex [format %02x $number]}
puts "quote-abs	$code	$hex	$x"
set x NONE
set expression "abs(\[set x)"
set code [catch {expr $expression} value]
set hex {};foreach ch [split $value {}] {scan $ch %c number;append hex [format %02x $number]}
puts "bracket-abs	$code	$hex	$x"
set x NONE
set expression "1 + \[set x \${bad])"
set code [catch {expr $expression} value]
set hex {};foreach ch [split $value {}] {scan $ch %c number;append hex [format %02x $number]}
puts "variable-brace	$code	$hex	$x"
