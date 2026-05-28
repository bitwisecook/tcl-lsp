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

MyProcCall $var1 \
           $var2 \
           $var3

set things {
    apple
    banana
}

#region Config
set a 1
set b 2
#endregion

package require Tcl
package require http
