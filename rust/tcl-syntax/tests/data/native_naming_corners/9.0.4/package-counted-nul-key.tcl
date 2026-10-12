set code [catch {set name "Pkg[format %c 0]Tail"; package provide $name 1.0; list [package present $name] [catch {package present Pkg}]} result]
binary scan $result H* hex
puts [list $code $hex]
