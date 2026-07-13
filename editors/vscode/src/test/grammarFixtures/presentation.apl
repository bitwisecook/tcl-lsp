# Marked-up grammar fixture for an iApp APL presentation. See #903.
# Assertion lines are `#` comments, so this stays a valid APL file.

#include "f5.apl_common"
#^^^^^^^ keyword.control.directive.apl

# `define <base-type> <new-name>` — the SECOND word is the type being reused and
# the THIRD is the new name. Scoping `choice` as the name here is a real bug that
# both this grammar's first draft and bitwisecook/vscode-iApp shipped.
define choice lb_method {
#^^^^^ keyword.other.define.apl
#      ^^^^^^ keyword.other.apl
#             ^^^^^^^^^ entity.name.function.apl
    "Round Robin" => "round-robin"
#                 ^^ keyword.operator.mapping.apl
}

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
    string label display "Number"
#                         ^^^^^^ string.quoted.double.apl
    message "Welcome $user."
#   ^^^^^^^ keyword.other.apl
#                    ^^^^^ variable.other.apl
    optional ( basic.protocol == "tcp" ) {
#   ^^^^^^^^ keyword.control.optional.apl
#              ^^^^^^^^^^^^^^ variable.other.field.apl
#                             ^^ keyword.operator.apl
        editchoice cipher default "DEFAULT"
#       ^^^^^^^^^^ keyword.other.apl
    }
}
