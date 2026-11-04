# Tcl execution evidence

## Question

> Option-looking trailing values are data when the selected string equal form reserves its two positional operands. Shortening every matching word could alter those values while a selected option prefix remains legal.

> For six fixed ASCII string equal argv forms, can the selected subcommand/option be shortened while the two reserved trailing values remain exact data in C Tcl 8.4–9.1 and current Jim?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> Six fixed ASCII source-file argv forms per provider, constructed by Tcl list and invoked through uplevel #0 in the original native process. Exact queried versions, executable/build/source/stream SHA joins and caught results are retained. No raw/counting operand, physical object, compiler admission, cache, storage, observer, Native completion, opaque Unicode or BIG-IP result is established.

## Script

```tcl-visible
puts "VERSION|[info patchlevel]"
foreach {id argv} [list \
 baseline_optionlike_tail [list string equal -nocase -nocase -nocase] \
 shortened_selected_prefix [list string e -n -nocase -nocase] \
 corrupted_tail [list string e -n -n -nocase] \
 two_literal_tails [list string equal -nocase -nocase] \
 corrupted_first_literal [list string equal -n -nocase] \
 selected_length_before_tail [list string equal -nocase -length 3 -nocase -nocase]] {
 set code [catch {uplevel #0 $argv} result]
 binary scan $result H* hex
 puts "$id|$code|$hex"
}
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: ASCII source-file CLI; Tcl-list quoted exact whole argv through uplevel #0
- Exit status: 0
- Standard output:

```text-visible
VERSION|8.4.20
baseline_optionlike_tail|0|31
shortened_selected_prefix|0|31
corrupted_tail|0|30
two_literal_tails|0|31
corrupted_first_literal|0|30
selected_length_before_tail|0|31
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: ASCII source-file CLI; Tcl-list quoted exact whole argv through uplevel #0
- Exit status: 0
- Standard output:

```text-visible
VERSION|8.5.19
baseline_optionlike_tail|0|31
shortened_selected_prefix|0|31
corrupted_tail|0|30
two_literal_tails|0|31
corrupted_first_literal|0|30
selected_length_before_tail|0|31
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: ASCII source-file CLI; Tcl-list quoted exact whole argv through uplevel #0
- Exit status: 0
- Standard output:

```text-visible
VERSION|8.6.18
baseline_optionlike_tail|0|31
shortened_selected_prefix|0|31
corrupted_tail|0|30
two_literal_tails|0|31
corrupted_first_literal|0|30
selected_length_before_tail|0|31
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: ASCII source-file CLI; Tcl-list quoted exact whole argv through uplevel #0
- Exit status: 0
- Standard output:

```text-visible
VERSION|9.0.4
baseline_optionlike_tail|0|31
shortened_selected_prefix|0|31
corrupted_tail|0|30
two_literal_tails|0|31
corrupted_first_literal|0|30
selected_length_before_tail|0|31
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: ASCII source-file CLI; Tcl-list quoted exact whole argv through uplevel #0
- Exit status: 0
- Standard output:

```text-visible
VERSION|9.1.0
baseline_optionlike_tail|0|31
shortened_selected_prefix|0|31
corrupted_tail|0|30
two_literal_tails|0|31
corrupted_first_literal|0|30
selected_length_before_tail|0|31
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: ASCII source-file CLI; Tcl-list quoted exact whole argv through uplevel #0
- Exit status: 0
- Standard output:

```text-visible
VERSION|0.84-9-g5bac7c9
baseline_optionlike_tail|0|31
shortened_selected_prefix|0|31
corrupted_tail|0|30
two_literal_tails|0|31
corrupted_first_literal|0|30
selected_length_before_tail|0|31
```

- Standard error: empty.
