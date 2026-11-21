# TclOO, class-system and dispatch probe

Fixtures behind queue items 2 and 3 of the [signed scan and precedence handover](../signed-scan-precedence.md). `bodies/` puts a signed-byte comparison and a precedence hazard in every body form; `discriminating/` holds minimal variants that separate causes (analyser output only, in `discriminating/analyser.out`). `variables/`, `dispatch/` and `dynamic/` (non-OO evaluation, namespaces and aliases) are programs that run cleanly, with each wrong call wrapped in `catch` and marked `# BAD`. The `.out` files are the analyser's output on `d6bd5cf7`.

Run a fixture under every release (TclOO needs 8.6 or later; `oo::configurable`, `classmethod`, `private`, `link`, `callback` and `initialise` need 9.0):

```bash
for f in dispatch/*.tcl; do for v in 8.6 9.0 9.1; do echo "$f $v: $(tclsh$v $f 2>&1 | tr '\n' ' ')"; done; done
```

itcl 4.3.5 for Tcl 9 is bundled with the Google Cloud SDK in this container; prepend `lappend auto_path /opt/google-cloud-sdk/platform/bundledpythonunix/lib` to the script and export `ITCL_LIBRARY=/opt/google-cloud-sdk/platform/bundledpythonunix/lib/itcl4.3.5`. snit and critcl are not installed, so their rows are static only.

There is no Tk for the shell interpreters here, but Python's tkinter carries Tcl/Tk 9.0.3: run a `tk/` fixture with `xvfb-run -a python3 -I tk/run_tk.py tk/t01-basic-layout.tcl` (it prints each `puts` and any error). Tk 8.6 rows are static only.

To reproduce the `.out` files, add this test temporarily to any `tcl-compiler` test module that imports `Analyser`, and run `OO_PROBE=<dir> cargo test -p tcl-compiler --lib -- --ignored oo_body_probe --nocapture`:

```rust
#[test]
#[ignore = "probe"]
fn oo_body_probe() {
    let dir = std::env::var("OO_PROBE").unwrap();
    let mut files: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).collect();
    files.sort();
    for path in files {
        let source = std::fs::read_to_string(&path).unwrap();
        for dialect in ["tcl8.6", "tcl9.0"] {
            let mut codes: Vec<String> = Analyser::new()
                .analyse(&source, dialect)
                .diagnostics
                .into_iter()
                .map(|d| {
                    let line = source[..d.span.as_range().start].matches('\n').count() + 1;
                    format!("[{:?} L{line} {}]", d.code, d.message.chars().take(70).collect::<String>())
                })
                .collect();
            codes.sort();
            println!("PROBE {} {dialect}: {}", path.file_name().unwrap().to_string_lossy(), codes.join(" "));
        }
    }
}
```
