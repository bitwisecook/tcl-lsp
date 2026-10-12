package require tcltest
namespace import -force ::tcltest::*
::tcltest::configure -verbose {body error start}
set ::contexts {}
test lifecycle-context {} -body {
    namespace eval ::testing {
        namespace eval abc {proc xyz {} {}}
        namespace eval def {proc xyz {} {}}
        trace add command abc::xyz delete {lappend ::contexts [list CB abc [namespace current] [info level] [namespace exists ::testing::def]]; namespace delete ::testing::def {}; #}
        trace add command def::xyz delete {lappend ::contexts [list CB def [namespace current] [info level] [namespace exists ::testing::abc]]; namespace delete ::testing::abc {}; #}
    }
    namespace delete ::testing
} -result {}
::tcltest::cleanupTests
puts [list CONTEXTS $::contexts]
