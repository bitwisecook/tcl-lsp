source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup [binary format H* 6f6f3a3a636c61737320637265617465203a3a6e68636c617373207b0a202020206d6574686f6420656d6974207b76616c75657d207b6c697374206f6b202476616c75657d0a202020206d6574686f6420707265666978207b7d207b6c697374205b73656c665d20656d6974207b6120627d7d0a7d0a736574206f626a656374205b3a3a6e68636c617373206e65775d]
set question [binary format H* 6576616c205b246f626a656374207072656669785d]
set setup_code [catch $setup setup_result setup_options]
set question_code not-run
set question_result {}
set question_options {}
if {!$setup_code} {
    set question_code [catch $question question_result question_options]
}
foreach field {setup_code setup_result setup_options question_code question_result question_options} {
    binary scan [set $field] H* bytes
    puts [list FIELD $field $bytes]
}
puts [list JIMTEST [array get ::testinfo]]
