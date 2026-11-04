# Tcl execution evidence

## Question

> A namespace-qualified procedure can be called while namespace eval remains active or after it returns. Using the namespace spelling alone to choose a procedure call-level expectation confuses these two actual script contexts; printed levels must remain separate from physical frame and Host metadata claims.

> For the exact original script, what public info level does the same procedure report when called inside namespace eval and when called directly by its namespace-qualified name after namespace eval returns?

## Assumptions

Assumptions were not separately recorded in the retained capture.

Recorded scope and limits:

> Exact three-line ordinary stdin script and two integer observations per recorded CLI process: Tcl8.4.20,8.5.19,8.6.18,9.0.4,9.1.0 and Jim0.84-9-g5bac7c9. VERSION, stdout, empty stderr, original exit/duration and invocation-file pins are retained independently. Five C pins identify shell wrappers; downstream C ELF/library closure is unrecorded. Jim invocation ELF bytes round-trip exactly, but shared dependencies and full environment/build replay are unrecorded. No physical frame/header/pointer layout, interpreter/Scope ownership, Host metadata ABI, exceptional completion/catch behavior, worker admission, TMM or implementation equivalence is measured. The two current Rust controls supply source coverage only.

## Script

```tcl-visible
puts "VERSION|[info patchlevel]"
namespace eval scope {proc within {} {return [info level]}; puts "NESTED|[within]"}
puts "DIRECT|[scope::within]"
```

## Results

### tcl8.4

- Recorded version: 8.4.20
- Recorded input channel: Original ordinary script supplied on standard input to the recorded CLI invocation; printed info patchlevel and script-visible info level, with exact raw stdout/stderr.
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION|8.4.20
NESTED|2
DIRECT|1
```

- Standard error: empty.

### tcl8.5

- Recorded version: 8.5.19
- Recorded input channel: Original ordinary script supplied on standard input to the recorded CLI invocation; printed info patchlevel and script-visible info level, with exact raw stdout/stderr.
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION|8.5.19
NESTED|2
DIRECT|1
```

- Standard error: empty.

### tcl8.6

- Recorded version: 8.6.18
- Recorded input channel: Original ordinary script supplied on standard input to the recorded CLI invocation; printed info patchlevel and script-visible info level, with exact raw stdout/stderr.
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION|8.6.18
NESTED|2
DIRECT|1
```

- Standard error: empty.

### tcl9.0

- Recorded version: 9.0.4
- Recorded input channel: Original ordinary script supplied on standard input to the recorded CLI invocation; printed info patchlevel and script-visible info level, with exact raw stdout/stderr.
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION|9.0.4
NESTED|2
DIRECT|1
```

- Standard error: empty.

### tcl9.1

- Recorded version: 9.1.0
- Recorded input channel: Original ordinary script supplied on standard input to the recorded CLI invocation; printed info patchlevel and script-visible info level, with exact raw stdout/stderr.
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION|9.1.0
NESTED|2
DIRECT|1
```

- Standard error: empty.

### jim

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: Original ordinary script supplied on standard input to the recorded CLI invocation; printed info patchlevel and script-visible info level, with exact raw stdout/stderr.
- Exit status: Not recorded
- Standard output:

```text-visible
VERSION|0.84-9-g5bac7c9
NESTED|2
DIRECT|1
```

- Standard error: empty.
