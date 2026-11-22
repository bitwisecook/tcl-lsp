# Package fixtures

`p11-pkgindex.tcl` and `p12-tm.tcl` refer to `<fixtures>`: replace it with the absolute path of this directory (they add `lib/` to `auto_path` and `tm/` to `::tcl::tm::path`). `p13b`–`p13e` reproduce the VM's silent exit while loading `msgcat`. Run the engines with `TCL_LIBRARY` set to the selected release's `library` directory (for example `tmp/tcl9.0.4/library`); without it neither engine has `auto_path`, `package unknown` or `::tcl::tm`. `engines.out` holds the runs without `TCL_LIBRARY`; the discovery results with it are recorded in `package-design.md`.
