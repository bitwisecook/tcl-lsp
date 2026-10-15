proc hex {value} {binary scan $value H* result; return $result}
set {auto_index( p )} SPACE
set auto_index() EMPTY
set auto_index(::N::p) QUALIFIED
set {auto_index(a)b)} CLOSE
foreach key [list { p } {} ::N::p {a)b}] {
    set name "auto_index($key)"
    puts "KEY|[hex $key]|[hex [set $name]]"
}
