# Tcl naming pattern: tcltest-shared-lifecycle

## Question

Do tcltest setup, body and cleanup share the expected namespace variable state, and is the setup variable absent after cleanup?

This distinction informs command and variable resolution in the LSP, compiler and VM. The observations cover only this case and the stated environments.

## Assumptions

The executed wrapper separates setup from the final operation. A setup failure leaves that operation unreached. Each capture uses a fresh process with the recorded interpreter; LC_ALL was C. The Jim wrapper also sources the test helper named in the script; the result depends on that helper. Other assumptions were not separately recorded.

## Script

```tcl-visible
source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup [binary format H* 7061636b61676520726571756972652074636c746573740a6e616d657370616365206576616c203a3a6e68207b0a202020206e616d65737061636520696d706f7274203a3a74636c746573743a3a2a0a20202020736574206f62736572766564207b7d0a2020202074657374206e616d652d636f727075732d31207b7d202d7365747570207b0a2020202020202020736574207061796c6f6164207b6120627d0a202020207d202d626f6479207b0a20202020202020206c617070656e64206f62736572766564205b736574207061796c6f61645d0a202020207d202d636c65616e7570207b0a20202020202020206c617070656e64206f62736572766564205b736574207061796c6f61645d0a2020202020202020756e736574207061796c6f61640a202020207d202d726573756c74207b7b6120627d7d0a7d]
set question [binary format H* 6c69737420243a3a6e683a3a6f62736572766564205b696e666f20657869737473203a3a6e683a3a7061796c6f61645d]
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
```

## Results

### Jim 0.84-9-g5bac7c9

- Recorded version: Jim 0.84-9-g5bac7c9
- Recorded input channel: script supplied to the interpreter.
- Exit status: 0
- Standard output:

```text-visible
FIELD setup_code 30
FIELD setup_result {}
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD question_code 31
FIELD question_result 63616e2774207265616420223a3a6e683a3a6f62736572766564223a206e6f2073756368207661726961626c65
FIELD question_options 2d636f64652031202d6c6576656c2030202d6572726f72696e666f207b7b7d207b7d2031207b7d7d202d6572726f72636f6465204e4f4e45
JIMTEST {verbose 0 numpass 1 stoponerror 0 template 0 numfail 0 numskip 0 numtests 1 reported 0 failed {} source input.tcl}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_code:

```text-visible
0⟦EOF⟧
```

- setup_result:

```text-visible
⟦EOF⟧
```

- setup_options:

```text-visible
-code 0 -level 0⟦EOF⟧
```

- question_code:

```text-visible
1⟦EOF⟧
```

- question_result:

```text-visible
can't read "::nh::observed": no such variable⟦EOF⟧
```

- question_options:

```text-visible
-code 1 -level 0 -errorinfo {{} {} 1 {}} -errorcode NONE⟦EOF⟧
```
