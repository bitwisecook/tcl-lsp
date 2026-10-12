source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup {}
set question [binary format H* 0a202020202020202070726f632074616b652078207b6c697374206f6c642024787d0a20202020202020206f6f3a3a636c617373206372656174652043207b666f72776172642073656e64203a3a74616b652066697865647d0a2020202020202020736574206f626a205b43206e65775d0a202020202020202072656e616d652074616b652073617665640a202020202020202070726f632074616b652078207b6c697374207265706c6163656d656e742024787d0a2020202020202020246f626a2073656e640a20202020]
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
