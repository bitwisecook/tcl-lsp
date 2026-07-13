# Marked-up grammar fixture for an iApp APL presentation. See #903.

section pool_config {
#^^^^^^ keyword.control.apl
#       ^^^^^^^^^^^ entity.name.section.apl
    string pool_name required
#   ^^^^^^ keyword.other.apl
#          ^^^^^^^^^ variable.other.field.apl
#                    ^^^^^^^^ entity.other.attribute-name.apl
    string addr validator "IpAddress"
#               ^^^^^^^^^ entity.other.attribute-name.apl
#                          ^^^^^^^^^ support.constant.validator.apl
    choice lb_method default "round-robin"
#   ^^^^^^ keyword.other.apl
#                    ^^^^^^^ entity.other.attribute-name.apl
    optional [ expr { $x > 1 } ]
#   ^^^^^^^^ keyword.control.optional.apl
#              ^^^^ support.function.tcl
#                     ^^ variable.other.tcl
}
