puts [list PROVIDER [info patchlevel] [info library] [info nameofexecutable]]
set script [binary format H* 736574206f62736572766564205b6c697374207b5d7d5d]
set code [catch {uplevel #0 $script} observed]
binary scan $observed H* observedHex
puts "braced-close|$code|$observedHex"
set script [binary format H* 736574206f62736572766564205b6c69737420225d225d]
set code [catch {uplevel #0 $script} observed]
binary scan $observed H* observedHex
puts "quoted-close|$code|$observedHex"
set script [binary format H* 736574206f62736572766564205b6c69737420613b2023205d2069676e6f7265640a6c69737420625d]
set code [catch {uplevel #0 $script} observed]
binary scan $observed H* observedHex
puts "comment-close|$code|$observedHex"
set script [binary format H* 736574206f62736572766564205b6c697374207b5b7d5d]
set code [catch {uplevel #0 $script} observed]
binary scan $observed H* observedHex
puts "braced-open|$code|$observedHex"
set script [binary format H* 736574206f62736572766564205b6c697374207b5d7d5d3a5b6c697374207a5d]
set code [catch {uplevel #0 $script} observed]
binary scan $observed H* observedHex
puts "concatenated|$code|$observedHex"
set script [binary format H* 736574206f62736572766564205b6c69737420223b225d]
set code [catch {uplevel #0 $script} observed]
binary scan $observed H* observedHex
puts "quoted-semicolon|$code|$observedHex"
set script [binary format H* 736574206f62736572766564205b6c69737420613b206c69737420625d]
set code [catch {uplevel #0 $script} observed]
binary scan $observed H* observedHex
puts "multiple|$code|$observedHex"
set script [binary format H* 736574206f62736572766564205b6c697374207b5d]
set code [catch {uplevel #0 $script} observed]
binary scan $observed H* observedHex
puts "incomplete|$code|$observedHex"
