oo::class create Factory {
    classmethod make {} { return [Factory new] }
}
oo::class create SubFactory {
    superclass Factory
}
puts "sub: [catch {SubFactory make} m] $m"
namespace eval ::a {
    oo::class create Factory {
        classmethod make {} { return 1 }
    }
    namespace export Factory
}
namespace eval ::b {
    namespace import ::a::Factory
    puts "import: [catch {Factory make} m] $m origin=[namespace origin Factory]"
}
