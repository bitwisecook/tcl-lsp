puts ENTER
namespace eval ::N {
    puts NAMESPACE
    variable value ready
    puts CREATED
    proc get {} {puts CALLED; variable value; puts LINKED; return $value}
}
puts DEFINED
puts [list RESULT [::N::get]]
