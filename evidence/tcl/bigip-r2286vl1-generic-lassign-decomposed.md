# BIG-IP variable lifecycle: generic-lassign-decomposed

## Question

How does lassign output assignment resolve and update a dynamically constructed decomposed Unicode name when the command is invoked through a computed command name? This constrains name resolution and output-variable semantics in the LSP, compiler and VM.

## Assumptions

The payload was evaluated under catch in an HTTP_REQUEST event, sequentially with 60 other cases in the same event frame. Earlier cases can affect later cases, so the results do not assume a fresh interpreter or unset variables. The script's own nested catch and hex encoding, where present, remain part of the result. The observations are limited to the recorded context and inputs. The immediately preceding case was [generic-lassign-binary_nul](bigip-r2286vl1-generic-lassign-binary-nul.md).

## Script

```tcl-visible
proc hex {s} {binary scan $s H* out; return $out}
namespace eval N {}
set n "e\u0301"
set c [catch {set h lassign; $h {A B} $n} r]
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
1 696e76616c696420636f6d6d616e64206e616d6520226c61737369676e22 1 0 3137⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The response from source port 42103 included this case in the sequential control matrix.

The script encoded its inner command result as hex; decoded bytes:

```text-visible
invalid command name "lassign"⟦EOF⟧
```

The script encoded its final variable-read result as hex; decoded bytes:

```text-visible
17⟦EOF⟧
```

### TMM 0:1, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:1, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
1 696e76616c696420636f6d6d616e64206e616d6520226c61737369676e22 1 0 3137⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The response from source port 42102 included this case in the sequential control matrix.

The script encoded its inner command result as hex; decoded bytes:

```text-visible
invalid command name "lassign"⟦EOF⟧
```

The script encoded its final variable-read result as hex; decoded bytes:

```text-visible
17⟦EOF⟧
```

### TMM 0:2, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:2, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
1 696e76616c696420636f6d6d616e64206e616d6520226c61737369676e22 1 0 3137⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The response from source port 42101 included this case in the sequential control matrix.

The script encoded its inner command result as hex; decoded bytes:

```text-visible
invalid command name "lassign"⟦EOF⟧
```

The script encoded its final variable-read result as hex; decoded bytes:

```text-visible
17⟦EOF⟧
```

### TMM 0:3, HTTP_REQUEST

- Recorded version: BIG-IP 21.1.0.1 build 0.0.26; Tcl patchlevel Not recorded in this run's result.
- Recorded input channel: TMM 0:3, HTTP_REQUEST.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
1 696e76616c696420636f6d6d616e64206e616d6520226c61737369676e22 1 0 3137⟦EOF⟧
```

The wrapper recorded Tcl completion code 0. The response from source port 42100 included this case in the sequential control matrix.

The script encoded its inner command result as hex; decoded bytes:

```text-visible
invalid command name "lassign"⟦EOF⟧
```

The script encoded its final variable-read result as hex; decoded bytes:

```text-visible
17⟦EOF⟧
```
