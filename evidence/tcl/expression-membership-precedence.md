# Tcl expression list membership precedence

## Question

How does `expr` group `in` and `ni` with numeric equality, string equality and relational comparisons? These cases distinguish groupings that affect evaluation, constant conditions and operator grouping advice.

## Assumptions

Each pinned C Tcl interpreter used its own library. The pinned Jim binary was built from the repository's reference revision for Jim. Each interpreter received the script as a file argument to `/dev/stdin` on 23 November 2026 with no script arguments. The list records expression order and uses `rejected` for a caught Tcl error. The reported exit status belongs to the interpreter process. Other inherited environment details were not recorded.

## Script

```tcl-visible
set a 2
set l {1}
set results {}
foreach expression {{$a == $a in $l} {"b" eq "b" in {1}} {"x" in {x} == 1} {1 != 1 ni {0}} {2 < 3 in {1}} {2 != 2 ni {0}}} {
    if {[catch {expr $expression} result]} { set result rejected }
    lappend results $result
}
puts [list [info patchlevel] $results]
```

## Results

### C Tcl 8.4.20

- Exit status: 0
- Standard output:

```text-visible
8.4.20 {rejected rejected rejected rejected rejected rejected}
```

- Standard error: empty.

### C Tcl 8.5.19

- Exit status: 0
- Standard output:

```text-visible
8.5.19 {1 1 1 0 1 0}
```

- Standard error: empty.

### C Tcl 8.6.18

- Exit status: 0
- Standard output:

```text-visible
8.6.18 {1 1 1 0 1 0}
```

- Standard error: empty.

### C Tcl 9.0.4

- Exit status: 0
- Standard output:

```text-visible
9.0.4 {1 1 1 0 1 0}
```

- Standard error: empty.

### C Tcl 9.1.0

- Exit status: 0
- Standard output:

```text-visible
9.1.0 {1 1 1 0 1 0}
```

- Standard error: empty.

### Jim 0.84-15-g2f8e155

- Exit status: 0
- Standard output:

```text-visible
0.84-15-g2f8e155 {1 1 0 0 1 0}
```

- Standard error: empty.
