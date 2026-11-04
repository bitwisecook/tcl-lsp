# Tcl naming pattern: computed-qualified

## Question

Can a qualified command name assembled from a namespace variable be invoked as a computed command head, and which command does that spelling reach?

This distinction informs command and variable resolution in the LSP, compiler and VM. The observations cover only this case and the stated environments.

## Assumptions

The executed wrapper separates setup from the final operation. A setup failure leaves that operation unreached. Each capture uses a fresh process with the recorded interpreter; LC_ALL was C. The Jim wrapper also sources the test helper named in the script; the result depends on that helper. Other assumptions were not separately recorded.

## Script

```tcl-visible
source [binary format H* 2f776f726b73706163652f74636c2d6c73702f746d702f6a696d74636c2d6f7261636c652f74657374732f74657374696e672e74636c]
set setup [binary format H* 6e616d657370616365206576616c203a3a6e68207b70726f6320656d6974207b7d207b72657475726e2066697273747d7d0a736574206e73203a3a6e680a736574206865616420247b6e737d3a3a656d6974]
set question [binary format H* 2468656164]
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
FIELD setup_result 3a3a6e683a3a656d6974
FIELD setup_options 2d636f64652030202d6c6576656c2030
FIELD question_code 30
FIELD question_result 6669727374
FIELD question_options 2d636f64652030202d6c6576656c2030
JIMTEST {verbose 0 numpass 0 stoponerror 0 template 0 numfail 0 numskip 0 numtests 0 reported 0 failed {} source input.tcl}
```

- Standard error: empty.

The following entries decode the FIELD payloads in standard output.

- setup_code:

```text-visible
0⟦EOF⟧
```

- setup_result:

```text-visible
::nh::emit⟦EOF⟧
```

- setup_options:

```text-visible
-code 0 -level 0⟦EOF⟧
```

- question_code:

```text-visible
0⟦EOF⟧
```

- question_result:

```text-visible
first⟦EOF⟧
```

- question_options:

```text-visible
-code 0 -level 0⟦EOF⟧
```
