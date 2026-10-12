namespace eval a {
    namespace export f
    proc f {} {return OLD}
}
namespace eval b {namespace import ::a::f}
rename a::f a::saved
proc a::f {} {return NEW}
puts [b::f]
puts [namespace origin b::f]
