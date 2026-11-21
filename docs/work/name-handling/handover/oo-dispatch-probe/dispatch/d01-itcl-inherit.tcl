package require itcl
itcl::class Base {
    method greet {name} { return "hi $name" }
}
itcl::class Derived {
    inherit Base
    method greet {name} { return "[chain $name]!" }
}
Derived d
puts [d greet x]
puts [catch {d greet} m]:$m ;# BAD arity
puts [catch {d nosuch} m]:$m ;# BAD unknown method
