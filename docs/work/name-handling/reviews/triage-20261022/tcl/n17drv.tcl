# capture driver: speclib/command around the body
proc speclib {name ver body} { uplevel #0 $body }
proc command {name args} { puts "registered: $name" }
speclib edge 2.0 {
    set a x
    set a:b BAD
    set arr(x:b) good
    set arr(BAD) bad
    command $arr($a:b) {arity 0}
}
