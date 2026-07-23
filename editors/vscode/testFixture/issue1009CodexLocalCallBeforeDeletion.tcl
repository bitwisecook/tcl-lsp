proc bar {} { return global }
namespace eval foo {
    proc bar {} { return local }
    proc caller {} { return [bar] }
}
foo::caller
rename foo::bar {}
