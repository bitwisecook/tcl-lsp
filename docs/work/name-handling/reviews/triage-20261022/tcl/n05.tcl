namespace eval a {namespace export *; proc foo {} {return A}}
namespace eval b {
    proc foo {} {return B}
    namespace import -force ::a::*
    puts [foo]
}
