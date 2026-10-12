proc hx {v} {binary scan $v H* result; return $result}
set ctx [binary format H* {@CONTEXT@}]
set name [binary format H* {@NAME@}]
puts "[hx [namespace canonical $ctx $name]]|[hx [namespace qualifiers $name]]|[hx [namespace tail $name]]|[hx [namespace parent $name]]"
