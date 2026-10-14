proc Configure {name default description} {proc $name {x} [subst -nocommands {return $default}]}
Configure port 8080 {the port}
puts [port ignored]
