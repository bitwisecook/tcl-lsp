set code [catch {oo::class create C {method :odd {} {return VALUE}; export :odd}; C create object; object :odd} result]
binary scan $result H* hex
puts [list $code $hex]
