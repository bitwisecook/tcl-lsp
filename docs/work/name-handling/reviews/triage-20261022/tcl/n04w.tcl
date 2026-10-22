namespace eval Foo {
    proc bar {} {return 1}
    namespace export bar
}
namespace import ::Foo::*
puts [bar]
