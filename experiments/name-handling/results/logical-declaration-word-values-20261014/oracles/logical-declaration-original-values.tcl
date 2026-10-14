if {[catch {info patchlevel} version]} {catch {info version} version}
puts [list version $version]
set name {LiteralChild}
set body {return original}
set code [catch {proc $name {} $body} result]
puts [list declaration $code $result]
puts [list original-values $name $body]
set code [catch {LiteralChild} result]
puts [list child-call $code $result]
