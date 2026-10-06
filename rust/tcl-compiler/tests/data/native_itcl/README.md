# Native Itcl dispatcher observations

The provider is Itcl 4.3.2 at commit
`9098b1d7dff3d87f515668dbe57fcb7dc17d8990` from
<https://github.com/tcltk/itcl>, built separately against C Tcl 8.6.18, 9.0.4
and 9.1.0. Configure its clean checkout
with `./configure --with-tcl="$native_tcl_root/unix"`, then run `make`.
These observations do not attest an Itcl implementation on another engine.
The 48 protocol/hook observations agree across all three native builds.

`compiler-hooks-probe.c` reads the actual installed command's `compileProc`
pointer through Tcl's private header. The seven observed factory, private
parser, class and instance commands have no compiler hook. This is independent
of their return values and their TclOO implementation details.

```sh
native_tcl_root="$PWD/tmp/tcl8.6.18"
native_itcl_root=/absolute/path/to/itcl-4.3.2-checkout
native_itcl_probes="$PWD/rust/tcl-compiler/tests/data/native_itcl"
native_probe_build=$(mktemp -d)
cc -I"$native_tcl_root/generic" -I"$native_tcl_root/unix" \
    "$native_itcl_probes/compiler-hooks-probe.c" \
    "$native_tcl_root/unix/libtcl8.6.a" -Wl,--export-dynamic \
    -ldl -lz -lpthread -lm -o "$native_probe_build/hooks"
"$native_probe_build/hooks" "$native_tcl_root/library" \
    "$native_itcl_root/library" "$native_itcl_root/libitcl4.3.2.so" \
    > "$native_probe_build/hooks.txt"
diff -u "$native_itcl_probes/compiler-hooks-8.6.txt" "$native_probe_build/hooks.txt"
TCL_LIBRARY="$native_tcl_root/library" "$native_tcl_root/unix/tclsh" \
    "$native_itcl_probes/protocols.tcl" "$native_itcl_root/library" \
    "$native_itcl_root/libitcl4.3.2.so" > "$native_probe_build/protocols.txt"
diff -u "$native_itcl_probes/protocols-8.6.txt" "$native_probe_build/protocols.txt"
rm -rf "$native_probe_build"
```

For C 9, build the same clean Itcl source against the corresponding Tcl Unix
directory. Use `libtcl9.0.a`/`libtcl9.1.a` in the probe command and the produced
`libtcl9itcl4.3.2.so` for loading; compare against the matching `-9.0.txt` or
`-9.1.txt` files. A clean source tree avoids `VPATH` finding incompatible C 8.6
object files during an out-of-tree build.

`roster.tcl` captures the actual loaded command surface recursively, using
the same two library arguments as `protocols.tcl`. Compare its output with
`roster-8.6.txt`, `roster-9.0.txt` or `roster-9.1.txt`. C 8.6 installs 143
commands under `::itcl`; both measured C 9 builds add only `::itcl::build-info`.
The fixture includes that exact installed dependency on C 9 and preserves
preload core prerequisites and private execution-observer guards.

The nine Tcl observations use fresh interpreters. They establish named and
generated instance spellings, distinct class-scoped proc slots, relative return
names, constructor error/deletion, mutable metaclass dispatch, and private parser
replacement/execution traces. A constructor can return a name after deleting its
command. Returned-name advice therefore proves neither existence nor object
class. Execution observers on a private dependency can prevent the class from
being installed. The bounded source recipe declines immediate definitions,
unvalidated names/formals, missing prerequisites and observer uncertainty.

The portable controls are `provider_fixtures::itcl::tests` in `tcl-compiler`.
Their installed dispatcher proof is separate from TclOO object/manufacturer
receipts, constructor execution and opcode admission.
