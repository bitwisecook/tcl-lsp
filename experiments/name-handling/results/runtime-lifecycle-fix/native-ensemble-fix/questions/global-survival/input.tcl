namespace eval ::testing {
    namespace eval abc {proc xyz {} {}}
    namespace eval def {proc xyz {} {}}
    trace add command abc::xyz delete "namespace delete ::testing::def {}; #"
    trace add command def::xyz delete "namespace delete ::testing::abc {}; #"
}
namespace delete ::testing
puts [list [namespace exists ::testing] [info commands set]]
