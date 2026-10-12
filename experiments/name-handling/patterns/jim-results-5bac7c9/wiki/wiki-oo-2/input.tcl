source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup {}
set question [binary format H* 0a20202020202020206f6f3a3a636c617373206372656174652043207b6d6574686f642068696464656e207b7d207b72657475726e206f6b7d3b206d6574686f6420707265666978207b7d207b6e616d65737061636520636f6465207b6d792068696464656e7d7d7d0a2020202020202020736574206f626a205b43206e65775d0a202020202020202073657420707265666978205b246f626a207072656669785d0a2020202020202020246f626a2064657374726f790a20202020202020206361746368207b6576616c20247072656669787d0a20202020]
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
