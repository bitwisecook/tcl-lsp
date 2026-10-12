source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup {}
set question [binary format H* 0a20202020202020206f6f3a3a636c61737320637265617465204d657461207b7375706572636c617373206f6f3a3a636c6173733b206d6574686f64206e65772061726773207b72657475726e206f7264696e6172797d7d0a20202020202020204d6574612063726561746520430a20202020202020207365742076616c7565205b43206e65775d0a20202020202020206c697374202476616c7565205b696e666f206f626a65637420697361206f626a656374202476616c75655d0a20202020]
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
