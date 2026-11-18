# Captured interp alias prefixes: errors at the call

## Question

> When an `interp alias` captures a command prefix, does a call through the alias raise for a mistake written in the captured prefix (an invalid option, an unknown selector, a missing clause script, an unknown channel), or run successfully? This decides whether a diagnostic on a captured slot reports a real error at the call, which the LSP must keep (anchored on the captured word in the alias definition, without a fix), or advice the LSP may decline.

The script answers only for these six captured prefixes, each run once in a fresh child interpreter; it does not establish other prefixes or a call that never reaches the alias.

## Assumptions

Each case is evaluated with `$child eval` in a new child interpreter created by `interp create` and deleted afterwards, so the cases do not share commands or variables. The printed message is truncated to 61 characters. `interp create` is unavailable in Jim 0.84 (`interp` takes no arguments), so Jim was not run. No BIG-IP run was recorded.

## Script

```tcl-visible
foreach {label script} {
    regsub-command {interp alias {} replace {} regsub -command; replace x y z}
    string-selector {interp alias {} strings {} string lenght; strings abc}
    append-padded {set out {}; interp alias {} add {} append out { }; add}
    regexp-dash {interp alias {} test {} regexp -dashPattern; test value}
    if-no-script {interp alias {} branch {} if 1; branch}
    puts-channel {interp alias {} output {} puts bogus; output hello}
} {
    set child [interp create]
    set code [catch {$child eval $script} message]
    interp delete $child
    puts [list $label $code [string range $message 0 60]]
}
```

## Results

### tclsh8.4

- Recorded version: 8.4.20
- Recorded input channel: script file passed to `tclsh8.4`.
- Exit status: 0
- Standard output:

```text-visible
regsub-command 1 {bad switch "-command": must be -all, -nocase, -expanded, -lin}
string-selector 1 {bad option "lenght": must be bytelength, compare, equal, firs}
append-padded 0 { }
regexp-dash 1 {bad switch "-dashPattern": must be -all, -about, -indices, -i}
if-no-script 1 {wrong # args: no script following "1" argument}
puts-channel 1 {can not find channel named "bogus"}
```

- Standard error: empty.

### tclsh8.5

- Recorded version: 8.5.19
- Recorded input channel: script file passed to `tclsh8.5`.
- Exit status: 0
- Standard output:

```text-visible
regsub-command 1 {bad switch "-command": must be -all, -nocase, -expanded, -lin}
string-selector 1 {unknown or ambiguous subcommand "lenght": must be bytelength,}
append-padded 0 { }
regexp-dash 1 {bad switch "-dashPattern": must be -all, -about, -indices, -i}
if-no-script 1 {wrong # args: no script following "1" argument}
puts-channel 1 {can not find channel named "bogus"}
```

- Standard error: empty.

### tclsh8.6

- Recorded version: 8.6.18
- Recorded input channel: script file passed to `tclsh8.6`.
- Exit status: 0
- Standard output:

```text-visible
regsub-command 1 {bad option "-command": must be -all, -nocase, -expanded, -lin}
string-selector 1 {unknown or ambiguous subcommand "lenght": must be bytelength,}
append-padded 0 { }
regexp-dash 1 {bad option "-dashPattern": must be -all, -about, -indices, -i}
if-no-script 1 {wrong # args: no script following "1" argument}
puts-channel 1 {can not find channel named "bogus"}
```

- Standard error: empty.

### tclsh9.0

- Recorded version: 9.0.4
- Recorded input channel: script file passed to `tclsh9.0`.
- Exit status: 0
- Standard output:

```text-visible
regsub-command 0 y
string-selector 1 {unknown or ambiguous subcommand "lenght": must be cat, compar}
append-padded 0 { }
regexp-dash 1 {bad option "-dashPattern": must be -all, -about, -indices, -i}
if-no-script 1 {wrong # args: no script following "1" argument}
puts-channel 1 {can not find channel named "bogus"}
```

- Standard error: empty.

### tclsh9.1

- Recorded version: 9.1.0
- Recorded input channel: script file passed to `tclsh9.1`.
- Exit status: 0
- Standard output:

```text-visible
regsub-command 0 y
string-selector 1 {unknown or ambiguous subcommand "lenght": must be cat, compar}
append-padded 0 { }
regexp-dash 1 {bad option "-dashPattern": must be -all, -about, -indices, -i}
if-no-script 1 {wrong # args: no script following "1" argument}
puts-channel 1 {can not find channel named "bogus"}
```

- Standard error: empty.
