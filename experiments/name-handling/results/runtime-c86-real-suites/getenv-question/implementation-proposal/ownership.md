# Unexecuted five-file getter support contract

No Rust edit or Cargo build is authorised by this proposal. The measured target
is the selected native C8.6 worker on a POSIX host, with genuine ownership by
the two implemented support commands. The five proposed Rust owners are:

| File | Change and in-file checks |
| --- | --- |
| rust/tcl-platform/src/lib.rs | Optional byte environment view on Env; dependency-free trait, default absent capability |
| rust/tcl-host-native/src/lib.rs | Unix NativeEnv implementation reading actual current process bytes plus its existing per-host overrides; primitive fixed-input tests |
| rust/tcl-registry/src/commands/tcl/clock_.rs | C8.6-only getenv constructor admission and direct spec; measured NoHook independent of purity; selected-profile metadata tests |
| rust/tcl-registry/src/commands/tcl/mod.rs | Add the authored getter spec to the existing Tcl catalogue |
| runtime/rust/src/cmd_clock.rs | Existing byte/arity/refusal owner consumer; two real Rc pool registration owners; getter, backing and retirement regressions |

No new command parser or semantic utility is needed. ValueOps already owns
original native string materialisation and byte-exact result construction;
tcl-core-types already owns CString extent; shared CmdError owns refusal
transport; runtime usage and object-command publication already have owners.
The existing ParseFormatArgs cmd-core parser and generic interpreter stock
object-command/backing helpers need no edits. A future VM adapter can consume
the same environment capability and original byte owners without adding a
compiler/analyser command-name branch.

Env gains `native_environment_bytes() -> Option<&dyn NativeEnvironmentBytes>`;
the separate trait supplies `get_bytes(&[u8]) -> Option<Vec<u8>>`. A present
capability's None means an actual missing entry; Some(empty) means an existing
empty entry. The absent capability means the operation is unavailable, never
an empty environment value. The lookup returns owned bytes from the current
host view on every call, with no bootstrap snapshot or persistent read cache.
The command selects the original name's CString extent before this call.

NativeEnv is the actual per-host source. Valid Unicode keys consult its existing
override map first; other keys skip that map, and both fall through to
`std::env::var_os(OsStr::from_bytes(key))`, preserving every returned Unix byte.
No `from_utf8_lossy`, original-name repair, global environment mutation or new
host simulator is involved. Existing Unicode Env methods retain their interface.
Non-Unix NativeEnv keeps the default absent capability: Unix evidence grants no
Windows wide-character conversion. Actual runtime BrowserEnv, VM BrowserEnv
and WasiEnv inherit that default; their empty/Unicode maps do not authenticate
a POSIX process environment view. No adapter edits are needed.

The getter checks `check_entered_native_operation` after its host access and
before result publication. Absent byte capability becomes
`ValueError::CommandProtocolUnavailable("native environment byte getter")`,
transported through shared CmdError/report_obj_cmd_error as a host-only refusal.
Converting HostError::Unsupported through CmdError would create a catchable
guest error and is the wrong transport for this unavailable native operation.

Arity keeps `wrong_args_for_invocation(argv, b"name")`. Because this worker is
an ObjCommand, successful guest-error publication also calls the existing
`note_c_api_error()` when no host refusal is pending; otherwise invoke_obj_cmd
would reset the already published errorCode. This reuses the same stated-error
rule as the C API collection owner, with no new interpreter method.

Replace the current Box-only pool ownership with Rc<ClockLiteralPool>.
Rc::into_raw on two real clones supplies the same client-data address to the
actual ParseFormatArgs and getenv registrations. Their existing once-only
ObjCommand deletion callbacks each consume one raw Rc with Rc::from_raw. Drop
the constructor's local Rc after publication. Creation failure still releases
the consumed clone through OwnedRuntimeShim's existing deletion owner. The
getter ignores client data while its registration genuinely retains it.
Deleting either one leaves the pool's literal references; deleting the last
implemented worker releases them while returned lists retain originals. There
are no retained placeholders for the five absent workers and no claim of the
original seven-worker constructor or full lifetime conformance.

Fixed primitive NativeHost tests use a fresh real child process seeded with
benign ASCII, empty, missing, raw-0xff-name and raw-0xff-value entries. Test the
actual capability's bytes directly; then update a valid key through the real
Env::set override API and query again, checking host-view currency and distinct
per-host override scopes. No thread-unsafe std::env::set_var is used. Native
runtime tests create original name objects for counted NUL and raw byte keys,
observe raw result bytes, preserve seeded success errorCode, check dynamic
invocation arity, and confirm absence of guest env read traces. A real NativeHost
override changed after bootstrap distinguishes the getter from the stale guest
env snapshot. The normal guest env write/unset synchronisation gap remains
explicit; getter implementation alone does not repair it.

The actual BrowserHost adapter can check absent capability/refusal transport;
this is an adapter capability check, not a claim of browser POSIX conformance.
Selected C8.6 backing tests must compare each worker's registry shipped spec to
actual `RegisteredBacking::Builtin`, NoHook token recipe and unchanged relative
epochs. Other supported C profiles intentionally lack these newly admitted
workers in this bounded implementation. Reuse the exact existing-API assertion
in [the earlier proposal](../proposal.md), extending its identity list only
after getenv exists. The C9-only backing xtask proves no selected C8.6 parity.

Focused builds and fresh original all.tcl attempts begin only after the
coordinator releases the compiler baseline and grants the edit/build window.
All future failures/refusals must stay explicit; the five other workers,
clock map order, env variable hooks and F5 remain outside this bounded job.
