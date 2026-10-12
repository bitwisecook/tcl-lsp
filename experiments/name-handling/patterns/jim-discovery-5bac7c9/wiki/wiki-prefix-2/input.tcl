source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup {}
set question [binary format H* 0a202020202020202070726f632074616b652061726773207b72657475726e2024617267737d0a202020202020202070726f63206578747261207b7d207b72657475726e207363726970747d0a202020202020202073657420707265666978202274616b655c6e6578747261220a20202020202020206c697374205b7b2a7d247072656669785d205b6576616c20247072656669785d0a20202020]
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
