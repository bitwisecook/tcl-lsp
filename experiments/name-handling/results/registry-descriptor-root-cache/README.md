# Registry descriptor root coordinates

The actual public `native_compilation_for_registration` metadata query accepts a
checked constructed descriptor label. One optional global root marker may be
removed; every remaining byte belongs to that label. This channel establishes
metadata availability, not a live native registration, object, compiler hook or
frame.

`probe.rs` queries the sealed Registry library for C9.0. `probe-all-releases.rs`
queries the same public API for all five C profiles. The recorded library hash
and source seal identify the actual implementation. Neither harness modifies or
constructs a Native binding receipt. The captured metadata baseline returns
stock descriptors for over-root labels: `::::set` on C8.4–9.1,
`::::tcl::info::exists` on C8.5–9.1, and `::::oo::define::method` on C8.6–9.1.
Older unsupported private/TclOO descriptors remain absent.

The corrected Registry keeps the original descriptor bytes in its cache key.
Each uncached adapter removes one root marker, selects exact authored catalogue
rows and validates the descriptor label, and passes the original identity to
subsequent metadata adapters. It does not feed an already unrooted key into
another root-fallback parser. All independent availability, ambiguity, hook,
registration and frozen-generation prerequisites remain.

`source-current.json` seals 6,391 Rust/manifest inputs. `validation.json` and raw
filter output record 83 distinct passing Registry controls: descriptor/cache and
version six, vendor summaries two, tcllib eighteen, dictionary five, mathfunc
fourteen, taint twenty-six and model assembly twelve. The first Cargo control is
also included in the six descriptor/version tests and is counted once. All
sealed inputs remained byte-identical through execution. `descriptor-root-current.log`
retains compilation and the first actual public-query negative control. The
corrected control declines the over-root labels and retains ordinary rooted/bare
results and frozen-generation currency across all five C profiles.

These are actual Registry/store metadata controls. Interpreter execution,
original native compiler/Runtime/VM entry paths and full-workspace gates are
outside this result. C and Jim written-command interpretation is a separate
input purpose; the wider root spellings accepted by a real interpreter do not
change a constructed descriptor's exact label.
