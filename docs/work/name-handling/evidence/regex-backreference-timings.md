# Tcl regexp backreference timings

Tcl 8.6.18 and 9.0.4 both take longer to reject an odd-length run when a backreference follows a group containing nested repetition or repeated alternatives. The control without a backreference accepts the same subjects. The figures report observed time over three input lengths for each of W303’s parsed shapes.

| Pattern | Tcl 8.6.18 at 21 / 81 / 321 characters, µs | Tcl 9.0.4 at 21 / 81 / 321 characters, µs | Result |
| --- | ---: | ---: | --- |
| `^((a+)+)\1$` | 49.6 / 1,133.2 / 74,466.7 | 319.3 / 837.0 / 52,937.5 | No match |
| `^((a*)*)\1$` | 342.0 / 6,104.1 / 67,692.5 | 57.0 / 1,892.5 / 52,663.2 | No match |
| `^((a\|aa)+)\1$` | 101.7 / 587.3 / 65,804.5 | 36.1 / 595.9 / 56,006.9 | No match |
| `^(a+)+$` | 0.4 / 0.7 / 1.2 | 0.3 / 0.5 / 136.2 | Match |

Run the following script with each version’s `tclsh`. Each timing is the mean of 30 iterations in microseconds.

```tcl
puts [info patchlevel]
foreach pattern {{^((a+)+)\1$} {^((a*)*)\1$} {^((a|aa)+)\1$} {^(a+)+$}} {
    foreach n {21 81 321} {
        set input [string repeat a $n]
        set matched [regexp $pattern $input]
        puts [list $pattern $n $matched [lindex [time {regexp $pattern $input} 30] 0]]
    }
}
```
