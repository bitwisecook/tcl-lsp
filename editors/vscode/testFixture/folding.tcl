# Folding test fixture
proc greet {name} {
    set msg "Hello, $name"
    if {$name eq "World"} {
        puts "Generic greeting"
    } else {
        puts $msg
    }
}

namespace eval ::myns {
    proc helper {} {
        return 42
    }
}
