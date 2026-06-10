# Fixture for variable-completion VS Code tests.
# Exercises bare $name, ${name}, cross-namespace, upvar, foreach,
# dict for/update/with, uplevel #0, and array elements.

set ::globalvar 9
set "foo-bar" 1

namespace eval ::other {
    variable baz 3
}

namespace eval ::myns {
    variable foo 1

    proc helper {arg} {
        upvar 1 $arg local_alias
        global myglobal
        set local_var 10
        set arr(name) "alice"
        set arr(age) 42
        foreach item {a b c} {
            puts $item
        }
        dict for {k v} {x 1 y 2} {
            puts $k
        }
        set d {name bob age 50}
        dict with d {
            puts $name
        }
        uplevel #0 {
            set uplevel_var 99
        }
        puts $local_var
    }
}
