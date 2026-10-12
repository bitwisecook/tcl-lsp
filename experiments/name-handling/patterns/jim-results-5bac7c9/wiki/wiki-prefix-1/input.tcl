source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup {}
set question [binary format H* 0a202020202020202070726f632074616b65207b66697865642076616c75657d207b6c69737420246669786564202476616c75657d0a202020202020202073657420707265666978205b6c6973742074616b65207b783b206572726f7220696e6a65637465647d5d0a20202020202020206576616c205b6c696e73657274202470726566697820656e64207b6120627d5d0a20202020]
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
