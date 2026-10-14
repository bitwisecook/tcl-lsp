set "a" "A"
set "a:b" "AB"
set "caf" "PREFIX"
set "café" "UNICODE"
set "" "EMPTY"
set "scalar(open" "OPEN"
set "arr(key)" "ELEMENT"
set "a{b" "FIRST"
set "a{b}c" "NESTED"
set code [catch "list \$a:b" value]
puts [list isolated_colon $code $value]
set code [catch "list \$café" value]
puts [list unicode $code $value]
set code [catch "list \${}" value]
puts [list empty $code $value]
set code [catch "list \${scalar(open}" value]
puts [list open_scalar $code $value]
set code [catch "list \${arr(key)}" value]
puts [list combined_element $code $value]
set code [catch "list \${a{b}c}" value]
puts [list nested_brace $code $value]
set code [catch "list \$arr(key)" value]
puts [list separate_element $code $value]
