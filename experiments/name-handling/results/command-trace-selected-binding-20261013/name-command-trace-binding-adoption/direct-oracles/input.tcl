
            namespace eval A {
                proc original {} {return A}
                rename original :p
            }
            namespace eval A: {proc p {} {return B}}
            namespace eval D {}
            proc observer_b {old new operation} {}
            proc extra {old new operation} {}
            proc observer_a {old new operation} {
                namespace eval A: {
                    if {[trace info command p] ne {{rename observer_b}}} {error wrong-before}
                    trace add command p rename extra
                    if {[trace info command p] ne {{rename extra} {rename observer_b}}} {error wrong-added}
                    trace remove command p rename extra
                    if {[trace info command p] ne {{rename observer_b}}} {error wrong-removed}
                    if {[p] ne {B}} {error wrong-command}
                }
            }
            namespace eval A: {trace add command p rename observer_b}
            namespace eval A {trace add command :p rename observer_a}
            set before [namespace eval A: {trace info command p}]
            namespace eval A {rename :p ::D::m}
            list $before [namespace eval A: {trace info command p}] [trace info command ::D::m]
        
puts [list $before [namespace eval A: {trace info command p}] [trace info command ::D::m]]
