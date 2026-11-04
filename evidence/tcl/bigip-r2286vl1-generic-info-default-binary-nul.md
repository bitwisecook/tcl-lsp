# BIG-IP variable lifecycle: generic-info_default-binary_nul

## Question

How does info default output assignment resolve and update a dynamically constructed name containing NUL when the command is invoked through a computed command name? This constrains name resolution and output-variable semantics in the LSP, compiler and VM.

## Assumptions

The payload was evaluated under catch in an HTTP_REQUEST event, sequentially with 60 other cases in the same event frame. Earlier cases can affect later cases, so the results do not assume a fresh interpreter or unset variables. The script's own nested catch and hex encoding, where present, remain part of the result. The observations are limited to the recorded context and inputs. The immediately preceding case was [generic-info_default-index](bigip-r2286vl1-generic-info-default-index.md).

## Script

```tcl-visible
proc hex {s} {binary scan $s H* out; return $out}
namespace eval N {}
set n [binary format H* ff0078]
set c [catch {proc sample {{x DEFAULT}} {}; set h info; $h default sample x $n} r]
set exists [info exists $n]
set rc [catch {set $n} got]
set summary [list $c [hex $r] $exists $rc [hex $got]]
set summary
```

## Results

### TMM 0:0, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:0, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
0 31 1 0 44454641554c54⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The response from source port 42103 included this case in the sequential control matrix.

The script encoded its inner command result as hex; decoded bytes:

```text-visible
1⟦EOF⟧
```

The script encoded its final variable-read result as hex; decoded bytes:

```text-visible
DEFAULT⟦EOF⟧
```

### TMM 0:1, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:1, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
0 31 1 0 44454641554c54⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The response from source port 42102 included this case in the sequential control matrix.

The script encoded its inner command result as hex; decoded bytes:

```text-visible
1⟦EOF⟧
```

The script encoded its final variable-read result as hex; decoded bytes:

```text-visible
DEFAULT⟦EOF⟧
```

### TMM 0:2, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:2, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
0 31 1 0 44454641554c54⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The response from source port 42101 included this case in the sequential control matrix.

The script encoded its inner command result as hex; decoded bytes:

```text-visible
1⟦EOF⟧
```

The script encoded its final variable-read result as hex; decoded bytes:

```text-visible
DEFAULT⟦EOF⟧
```

### TMM 0:3, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:3, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
0 31 1 0 44454641554c54⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The response from source port 42100 included this case in the sequential control matrix.

The script encoded its inner command result as hex; decoded bytes:

```text-visible
1⟦EOF⟧
```

The script encoded its final variable-read result as hex; decoded bytes:

```text-visible
DEFAULT⟦EOF⟧
```
