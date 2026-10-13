proc emit {name value} {
    binary scan $value H* hex
    puts "$name=$hex"
}
set single {a\
 b}
set escaped {a\\
 b}
set preceding {a  \
	 b}
emit single $single
emit escaped $escaped
emit preceding $preceding
set native [format "A\\%c\n\t B" 13]
emit native-crlf [eval "set result {$native}"]
