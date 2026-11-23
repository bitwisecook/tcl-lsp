# TIP 602: ~ no longer expands in path names
puts "pathtype ~/x: [file pathtype ~/x]"
puts "split ~/x: [file split ~/x]"
puts "join a ~b: [file join a ~b]"
puts "tildeexpand: [catch {file tildeexpand ~/x} r] $r"
puts "home: [catch {file home} r] $r"
