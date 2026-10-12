source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup [binary format H* 6e616d657370616365206576616c203a3a6e68207b0a2020202070726f6320656d6974207b66697865642076616c75657d207b6c69737420246669786564202476616c75657d0a202020206e616d65737061636520656e73656d626c6520637265617465202d636f6d6d616e6420666163616465202d6d6170205b6c6973742073656e64205b6c697374203a3a6e683a3a656d6974207b6120627d5d5d0a7d]
set question [binary format H* 3a3a6e683a3a6661636164652073656e64207b6320647d]
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
