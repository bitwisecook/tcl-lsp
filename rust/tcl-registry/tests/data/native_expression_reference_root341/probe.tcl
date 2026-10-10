# Fixed ASCII LF source file. Each hex field is decoded by binary format H*
# into the reached expr argument or the exact declared variable name. This is
# an explicit constructed Tcl value channel, not opaque source-byte or native
# object/header/compiler-state admission. Public completion and result only.
puts [list META patchlevel [info patchlevel]]
proc expression_reference_probe {label name_hex expr_hex value} {
    set name [binary format H* $name_hex]
    set expression [binary format H* $expr_hex]
    set $name $value
    set code [catch [list expr $expression] result]
    binary scan $expression H* expression_bytes
    binary scan $result H* result_bytes
    puts [list EXPR label $label expression_hex $expression_bytes code $code result_hex $result_bytes]
    unset $name
}
expression_reference_probe unmatched_scalar_open 7363616c6172286f70656e 247b7363616c6172286f70656e7d 11
expression_reference_probe scalar_parenthesis_tail 7363616c6172286f70656e297461696c 247b7363616c6172286f70656e297461696c7d 12
expression_reference_probe combined_closed_element 617272286b657929 247b617272286b6579297d 21
expression_reference_probe separate_closed_element 617272286b657929 24617272286b657929 21
expression_reference_probe combined_multiple_parentheses 61727228696e6e657229286b657929 247b61727228696e6e657229286b6579297d 22
expression_reference_probe first_close_reference 617b62 247b617b627d 31
expression_reference_probe nested_close_reference 617b627d63 247b617b627d637d 32
expression_reference_probe literal_dollar_scalar 63617368246e616d65 247b63617368246e616d657d 41
