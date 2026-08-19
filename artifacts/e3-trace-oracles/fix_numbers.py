p = "/tmp/claude-0/-home-user-tcl-lsp/49cadaef-dfc0-5f80-9bf3-267da5a707d9/scratchpad/e3/pr-body.md"
s = open(p).read()

old_cmds = """cargo fmt --all --check
cargo clippy -p tcl-vm -p tcl-cmd-core -p tcl-registry --all-targets
TCL_LSP_TCLSH86=tmp/tcl8616-install/bin/tclsh8.6 \\
  cargo test -p tcl-vm -p tcl-cmd-core --no-fail-fast
cd runtime/rust && cargo fmt --all --check && cargo clippy --all-targets && cargo test --no-fail-fast
cargo xtask owner-resolution"""
new_cmds = """cargo fmt --all --check
cargo clippy -p tcl-vm -p tcl-cmd-core -p tcl-registry --all-targets
TCL_LSP_TCLSH86=tmp/tcl8616-install/bin/tclsh8.6 \\
  cargo test -p tcl-vm -p tcl-cmd-core -p tcl-registry --no-fail-fast
cd runtime/rust && cargo fmt --all --check && cargo clippy --all-targets && cargo test --no-fail-fast
make xtask-check"""
assert s.count(old_cmds) == 1
s = s.replace(old_cmds, new_cmds, 1)

old_res = """Results: format check clean; clippy clean on every crate touched; 1119 passed /
0 failed over 42 binaries for `tcl-vm` plus `tcl-cmd-core`; 534 passed / 0
failed for `runtime/rust`; owner-resolution OK. The `runtime/rust` format check
also reports four files this branch does not touch — that is the pre-existing
#1623 drift on `rust`, left alone deliberately."""
new_res = """Results on the current head: format check clean; clippy clean on every crate
touched; 1967 passed / 0 failed over 61 binaries for `tcl-vm` plus
`tcl-cmd-core` plus `tcl-registry`; 536 passed / 0 failed for `runtime/rust`;
every `xtask-check` drift gate green with no generated-file churn. The
`runtime/rust` format check also reports four files this branch does not touch
— that is the pre-existing #1623 drift on `rust`, left alone deliberately."""
assert s.count(old_res) == 1
s = s.replace(old_res, new_res, 1)

open(p, "w").write(s)
print("numbers refreshed")
