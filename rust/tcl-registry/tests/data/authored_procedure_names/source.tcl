namespace eval ::ns {}
set code [catch {namespace eval ::ns {proc :p {} {return X}}} value]
set hex {};foreach ch [split $value {}] {scan $ch %c number;append hex [format %02x $number]}
puts "local-colon\t$code\t$hex"
set code [catch {namespace eval :: {proc :p {} {return X}}} value]
set hex {};foreach ch [split $value {}] {scan $ch %c number;append hex [format %02x $number]}
puts "root-colon\t$code\t$hex"
set code [catch {namespace eval ::ns {proc p {} {return X}}} value]
set hex {};foreach ch [split $value {}] {scan $ch %c number;append hex [format %02x $number]}
puts "local-plain\t$code\t$hex"
