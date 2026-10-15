puts [list PROVIDER [info patchlevel] [info library] [info nameofexecutable]]
set script [binary format H* 736574206f62736572766564205b6c697374207b5d7d5d]
set code [catch {uplevel #0 $script} observed]
puts "braced-close|$code|[binary encode hex $observed]"
set script [binary format H* 736574206f62736572766564205b6c69737420225d225d]
set code [catch {uplevel #0 $script} observed]
puts "quoted-close|$code|[binary encode hex $observed]"
set script [binary format H* 736574206f62736572766564205b6c69737420613b2023205d2069676e6f7265640a6c69737420625d]
set code [catch {uplevel #0 $script} observed]
puts "comment-close|$code|[binary encode hex $observed]"
set script [binary format H* 736574206f62736572766564205b6c697374207b5b7d5d]
set code [catch {uplevel #0 $script} observed]
puts "braced-open|$code|[binary encode hex $observed]"
set script [binary format H* 736574206f62736572766564205b6c697374207b5d7d5d3a5b6c697374207a5d]
set code [catch {uplevel #0 $script} observed]
puts "concatenated|$code|[binary encode hex $observed]"
set script [binary format H* 736574206f62736572766564205b6c69737420223b225d]
set code [catch {uplevel #0 $script} observed]
puts "quoted-semicolon|$code|[binary encode hex $observed]"
set script [binary format H* 736574206f62736572766564205b6c69737420613b206c69737420625d]
set code [catch {uplevel #0 $script} observed]
puts "multiple|$code|[binary encode hex $observed]"
set script [binary format H* 736574206f62736572766564205b6c697374207b5d]
set code [catch {uplevel #0 $script} observed]
puts "incomplete|$code|[binary encode hex $observed]"
