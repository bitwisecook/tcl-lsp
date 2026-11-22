namespace eval ::a::lib { proc helper {} { return a-lib } }
namespace eval ::lib { proc helper {} { return top-lib } }
namespace eval ::a {
    namespace path lib
    proc p {} { return [helper] }
}
puts [::a::p]
