package require tcltest
namespace import -force ::tcltest::*
::tcltest::configure -verbose {body error start}
test namespace-56.2 {bug f97d4ee020: mutually-entangled deletion} {
    namespace eval ::testing {
	namespace eval abc {proc xyz {} {}}
	namespace eval def {proc xyz {} {}}
	trace add command abc::xyz delete "namespace delete ::testing::def {}; #"
	trace add command def::xyz delete "namespace delete ::testing::abc {}; #"
    }
    namespace delete ::testing
} {}
::tcltest::cleanupTests
