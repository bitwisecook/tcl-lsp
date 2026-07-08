# Issue #806 — report::defstyle scoped commands (top/data/columns/…); `toop` is a typo.
::report::defstyle simpletable {} {
    top set [split "x"]
    data set [split "y"]
    bottom enable
    topdatasep enable
    columns
    toop set z
}
