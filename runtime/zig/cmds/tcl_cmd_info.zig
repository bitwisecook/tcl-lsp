// Command: info — introspection into the interpreter state.
//
// Subcommands implemented:
//   info exists varName      — variable exists in the current frame?
//   info commands ?pattern?  — live commands reachable from the
//                              current ns's resolution chain (name
//                              walk + path + root).  Patterns may be
//                              qualified (``::foo::*``) or bare —
//                              both match against simple names.
//                              Hidden-table entries and dead import
//                              redirects are excluded.
//   info procs    ?pattern?  — same walk as ``info commands`` but
//                              filtered to interpreted procs
//                              (``func_idx == 0 && body_obj != 0``).
//                              Matches tclsh's "only interpreted
//                              procs are procs".
//   info body     procName   — body TclObj of an interpreted proc.
//   info args     procName   — params TclObj of an interpreted proc.
//   info default  proc arg var — write the default value of ``arg``
//                              into ``var`` and return 1 if the
//                              parameter has a default, else 0.
//   info complete script     — braces / brackets / quotes all
//                              balanced?  tcltest uses this to
//                              validate user-supplied scripts
//                              before evaluation.
//   info nameofexecutable    — placeholder.
//   info tclversion          — "8.6".
//   info patchlevel          — "8.6.15".
//   info hostname            — "wasm".
//
// All walkers use the shared 16-byte bucket layout
// (``[0..3]`` name_ptr, ``[4..7]`` name_len, ``[8..11]`` hash,
// ``[12..15]`` value = Command*) and read ``Namespace.cmd_table`` /
// ``Namespace.child_table`` inline — no allocation per probed
// bucket.  Result strings are built in one pass with a
// bump-allocated output buffer sized up front.

const obj = @import("../valtypes/tcl_obj.zig");
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_int = obj.obj_new_int;
const obj_new_string = obj.obj_new_string;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;
const alloc = obj.alloc;
const memcpy = obj.memcpy;

const frames = @import("../interp/tcl_frames.zig");
const procs = @import("../interp/tcl_procs.zig");
const tcl_ns = @import("../interp/tcl_ns.zig");
const tcl_string = @import("../valtypes/tcl_string.zig");

const str_eq = @import("../valtypes/tcl_chars.zig").str_eq;

const BUCKET_SIZE: u32 = 16;

// -- Small exports preserved for the interpreter dispatch -----------------

/// info exists varName — returns 1 if variable is defined, 0 otherwise.
/// Checks current frame locals first, then globals.
pub export fn info_exists(name: i32) i32 {
    return frames.var_exists(name);
}

/// Raise ``"X" isn't a procedure`` with the ``TCL LOOKUP PROCEDURE X``
/// error code path that ``info body`` / ``info args`` /
/// ``info default`` all share when the name doesn't resolve to an
/// interpreted proc.  Matches the Tcl 9 wording verbatim.
fn raise_not_a_procedure(name_ptr: u32, name_len: u32) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const prefix = "\"";
    const suffix = "\" isn't a procedure";
    const total: u32 = @as(u32, @intCast(prefix.len)) + name_len + @as(u32, @intCast(suffix.len));
    const buf = alloc(total);
    const d: [*]u8 = @ptrFromInt(buf);
    d[0] = '"';
    if (name_len > 0) {
        const sp: [*]const u8 = @ptrFromInt(name_ptr);
        for (0..name_len) |k| d[prefix.len + k] = sp[k];
    }
    for (suffix, 0..) |b, k| d[prefix.len + name_len + k] = b;
    const msg = obj_new_string(@bitCast(buf), @bitCast(total));
    catch_mod.tcl_cmd_error(msg);
}

/// Resolve a name to its Command and verify it's an interpreted
/// proc.  On miss (unknown / alias / hidden / compiled proc), raise
/// ``"X" isn't a procedure`` and return 0.  Shared by
/// body / args / default so the error surface stays uniform.
///
/// ``proc_lookup`` transparently unwraps ``CMD_IMPORTED`` to the
/// terminal source Command, so imports that point at a real
/// interpreted proc succeed here.  Aliases don't unwrap (by design
/// — ``CMD_ALIAS`` is preserved for introspection) and fail the
/// filter.  Compiled procs (``func_idx != 0``) have no body TclObj
/// so they fail the ``body == 0`` check, matching Tcl's "no
/// retrievable body" rule.
fn resolve_interpreted_proc(name: i32) u32 {
    const sn = obj_ensure_string(name);
    const bucket = procs.proc_lookup(name);
    if (bucket == 0) {
        raise_not_a_procedure(sn.ptr, sn.len);
        return 0;
    }
    const cmd: u32 = @bitCast(bucket);
    const flags: u32 = @bitCast(read_i32(cmd + procs.OFF_FLAGS));
    if ((flags & procs.CMD_ALIAS) != 0) {
        raise_not_a_procedure(sn.ptr, sn.len);
        return 0;
    }
    const func_idx = read_i32(cmd + procs.OFF_FUNC_IDX);
    if (func_idx != 0) {
        raise_not_a_procedure(sn.ptr, sn.len);
        return 0;
    }
    const body = procs.proc_get_body(bucket);
    if (body == 0) {
        raise_not_a_procedure(sn.ptr, sn.len);
        return 0;
    }
    return cmd;
}

/// info body procName — returns the body TclObj for an interpreted
/// proc.  Raises ``"X" isn't a procedure`` for anything else
/// (missing, alias, hidden, compiled) and returns an empty TclObj
/// so the error-flag path is the authoritative signal.
pub export fn info_body(name: i32) i32 {
    // Compiled procs (``func_idx != 0``) had their original Tcl body
    // lifted into a WASM function at compile time, but the codegen
    // prologue also stashes the source-text body via
    // ``proc_set_body_source`` so callers like
    // ``test foo {} -body [info body bar] …`` can re-eval the body
    // through the interpreter.  When the stash is present we return
    // it; when it's missing (synthetic procs that have no Tcl-source
    // equivalent — e.g. ``when`` handlers, or older bundles built
    // before the stash existed) we fall back to the empty string so
    // the surrounding suite still reaches its tcltest summary line
    // instead of trapping.
    const sn = obj_ensure_string(name);
    const bucket = procs.proc_lookup(name);
    if (bucket == 0) {
        raise_not_a_procedure(sn.ptr, sn.len);
        return 0;
    }
    const cmd: u32 = @bitCast(bucket);
    const flags: u32 = @bitCast(read_i32(cmd + procs.OFF_FLAGS));
    if ((flags & procs.CMD_ALIAS) != 0) {
        raise_not_a_procedure(sn.ptr, sn.len);
        return 0;
    }
    const body = procs.proc_get_body(bucket);
    const func_idx = read_i32(cmd + procs.OFF_FUNC_IDX);
    if (func_idx != 0) {
        if (body != 0) return body;
        return obj_new_string(0, 0);
    }
    if (body == 0) {
        raise_not_a_procedure(sn.ptr, sn.len);
        return 0;
    }
    return body;
}

/// info args procName — returns the params TclObj for an
/// interpreted proc.  Mirrors :func:`info_body`'s tolerance for
/// compiled procs: when ``func_idx != 0`` the AOT path may have
/// pre-registered the proc with ``params_obj == 0`` even though a
/// later interpreted ``proc`` re-registration restored the params,
/// or vice versa.  We accept any non-CMD_ALIAS bucket and return
/// whatever ``params_obj`` holds, falling back to an empty list when
/// the slot is null (matches ``info body``'s "return empty" branch).
/// Used by ``rename.test`` 5.1 (``info args unknown.old``) and
/// any ``test ... -body [list args [info args FOO]]`` shape.
pub export fn info_args(name: i32) i32 {
    const sn = obj_ensure_string(name);
    const bucket = procs.proc_lookup(name);
    if (bucket == 0) {
        raise_not_a_procedure(sn.ptr, sn.len);
        return 0;
    }
    const cmd: u32 = @bitCast(bucket);
    const flags: u32 = @bitCast(read_i32(cmd + procs.OFF_FLAGS));
    if ((flags & procs.CMD_ALIAS) != 0) {
        raise_not_a_procedure(sn.ptr, sn.len);
        return 0;
    }
    if ((flags & procs.CMD_BUILTIN_FORWARD) != 0 or
        (flags & procs.CMD_BUILTIN_MASKED) != 0)
    {
        raise_not_a_procedure(sn.ptr, sn.len);
        return 0;
    }
    const params = procs.proc_get_params(bucket);
    if (params == 0) return obj_new_string(0, 0);
    return params;
}

/// ``info complete script`` — returns 1 iff the script can be
/// safely passed to ``eval`` without the parser running out of
/// source mid-construct (mid-brace, mid-quote, mid-bracket,
/// mid-``${name}``, mid-``$arr(idx)``, mid-comment-line-continuation,
/// or trailing ``\<NL>`` line continuation).
///
/// Mirrors reference Tcl's ``Tcl_CommandComplete`` — which drives
/// ``Tcl_ParseCommand`` over the full source and reports ``1`` whenever
/// ``parse.incomplete`` is **not** set, even if a parse error like
/// "extra characters after close-brace" was raised.  The implementation
/// here walks the script with a state machine that tracks word
/// boundaries (``{`` / ``"`` / ``[`` / ``$`` only act as openers at
/// word start), backslash escapes (``\X`` consumes two bytes; ``\<NL>``
/// at the very end is a continuation that signals incomplete), and
/// comment processing (``#`` at command start runs to the next
/// unescaped newline; ``\<NL>`` inside a comment extends the line).
pub fn info_complete(script: i32) i32 {
    const s = obj_ensure_string(script);
    if (s.len == 0) return obj_new_int(1);
    const sp: [*]const u8 = @ptrFromInt(s.ptr);
    return obj_new_int(if (check_complete(sp, s.len)) 1 else 0);
}

fn check_complete(sp: [*]const u8, len: u32) bool {
    var p: u32 = 0;
    while (p < len) {
        // Inter-command whitespace + line continuations.
        while (p < len) {
            const c = sp[p];
            if (c == ' ' or c == '\t' or c == '\n' or c == '\r' or c == ';') {
                p += 1;
            } else if (c == '\\' and p + 1 < len and sp[p + 1] == '\n') {
                p += 2;
            } else {
                break;
            }
        }
        if (p >= len) break;
        // Comment at command start: consume to next unescaped ``\n``;
        // ``\<X>`` inside a comment skips two bytes (so a literal
        // ``\<NL>`` extends the comment line).
        if (sp[p] == '#') {
            while (p < len) {
                if (sp[p] == '\\' and p + 1 < len) {
                    p += 2;
                } else if (sp[p] == '\n') {
                    p += 1;
                    break;
                } else {
                    p += 1;
                }
            }
            continue;
        }
        // Parse one word.
        if (sp[p] == '{') {
            const next = skip_braced_complete(sp, p, len) orelse return false;
            p = next;
            // "Extra characters after close-brace" is a syntax error
            // but not an incomplete one — Tcl's CommandComplete returns
            // 1.  Stop scanning so we don't miscount the trailing junk.
            if (p < len) {
                const c = sp[p];
                if (c != ' ' and c != '\t' and c != '\n' and c != '\r' and c != ';') {
                    return true;
                }
            }
        } else if (sp[p] == '"') {
            const next = skip_quoted_complete(sp, p, len) orelse return false;
            p = next;
            if (p < len) {
                const c = sp[p];
                if (c != ' ' and c != '\t' and c != '\n' and c != '\r' and c != ';') {
                    return true;
                }
            }
        } else {
            // Bare word: scan until whitespace / separator / line
            // continuation.  Inside, brackets must close, ``${name}``
            // must close, ``$arr(idx)`` must close, ``\X`` consumes 2.
            while (p < len) {
                const c = sp[p];
                if (c == ' ' or c == '\t' or c == '\n' or c == '\r' or c == ';') break;
                if (c == '\\' and p + 1 < len) {
                    if (sp[p + 1] == '\n') break; // line continuation = word terminator
                    p += 2;
                } else if (c == '[') {
                    const next = skip_bracket_complete(sp, p, len) orelse return false;
                    p = next;
                } else if (c == '$' and p + 1 < len) {
                    p += 1;
                    if (sp[p] == '{') {
                        p += 1;
                        while (p < len and sp[p] != '}') {
                            if (sp[p] == '\\' and p + 1 < len) p += 2 else p += 1;
                        }
                        if (p >= len) return false;
                        p += 1;
                    } else {
                        while (p < len) {
                            const cc = sp[p];
                            if ((cc >= 'a' and cc <= 'z') or
                                (cc >= 'A' and cc <= 'Z') or
                                (cc >= '0' and cc <= '9') or cc == '_')
                            {
                                p += 1;
                            } else if (cc == ':' and p + 1 < len and sp[p + 1] == ':') {
                                p += 2;
                            } else {
                                break;
                            }
                        }
                        if (p < len and sp[p] == '(') {
                            p += 1;
                            while (p < len and sp[p] != ')') {
                                if (sp[p] == '\\' and p + 1 < len) p += 2 else p += 1;
                            }
                            if (p >= len) return false;
                            p += 1;
                        }
                    }
                } else {
                    p += 1;
                }
            }
        }
    }
    // Trailing line continuation: source ends with an UN-escaped
    // ``\<NL>`` (odd number of preceding backslashes).  parse-15.39 /
    // 15.45 / 15.50 — the ``\<NL>`` signals the command continues to
    // a line that isn't there, so the script is incomplete.
    if (len >= 2 and sp[len - 1] == '\n') {
        var bs_count: u32 = 0;
        var j: u32 = len - 1;
        while (j > 0) {
            j -= 1;
            if (sp[j] == '\\') bs_count += 1 else break;
        }
        if (bs_count % 2 == 1) return false;
    }
    return true;
}

fn skip_braced_complete(sp: [*]const u8, start: u32, len: u32) ?u32 {
    var p = start + 1;
    var depth: u32 = 1;
    while (p < len and depth > 0) {
        if (sp[p] == '\\' and p + 1 < len) {
            p += 2;
            continue;
        }
        if (sp[p] == '{') depth += 1 else if (sp[p] == '}') depth -= 1;
        p += 1;
    }
    if (depth != 0) return null;
    return p;
}

fn skip_quoted_complete(sp: [*]const u8, start: u32, len: u32) ?u32 {
    var p = start + 1;
    while (p < len and sp[p] != '"') {
        if (sp[p] == '\\' and p + 1 < len) {
            p += 2;
        } else if (sp[p] == '[') {
            const next = skip_bracket_complete(sp, p, len) orelse return null;
            p = next;
        } else {
            p += 1;
        }
    }
    if (p >= len) return null;
    return p + 1;
}

fn skip_bracket_complete(sp: [*]const u8, start: u32, len: u32) ?u32 {
    var p = start + 1;
    var depth: u32 = 1;
    // Whether the next non-whitespace byte starts a fresh command —
    // i.e., we're at the start of the bracket or just past ``\n`` /
    // ``;``.  Comments (``#``) only act at command-start positions; a
    // ``#`` mid-word is a literal byte.  parse-6.10 puts ``]`` inside a
    // ``# this is a comment ]`` line which must NOT close the bracket.
    var at_cmd_start: bool = true;
    while (p < len and depth > 0) {
        if (sp[p] == '\\' and p + 1 < len) {
            p += 2;
            continue;
        }
        const c = sp[p];
        if (c == '\n' or c == ';') {
            at_cmd_start = true;
            p += 1;
            continue;
        }
        if (c == ' ' or c == '\t' or c == '\r') {
            p += 1;
            continue;
        }
        if (at_cmd_start and c == '#') {
            // Comment line — consume to next un-escaped ``\n``.
            while (p < len) {
                if (sp[p] == '\\' and p + 1 < len) {
                    p += 2;
                } else if (sp[p] == '\n') {
                    p += 1;
                    break;
                } else {
                    p += 1;
                }
            }
            at_cmd_start = true;
            continue;
        }
        at_cmd_start = false;
        if (c == '[') {
            depth += 1;
            p += 1;
        } else if (c == ']') {
            depth -= 1;
            p += 1;
        } else if (c == '{') {
            const next = skip_braced_complete(sp, p, len) orelse return null;
            p = next;
        } else if (c == '"') {
            const next = skip_quoted_complete(sp, p, len) orelse return null;
            p = next;
        } else {
            p += 1;
        }
    }
    if (depth != 0) return null;
    return p;
}

pub fn info_nameofexecutable() i32 {
    return obj.obj_new_string_copy(@intFromPtr("tcl_runtime.wasm".ptr), 16);
}

pub fn info_tclversion() i32 {
    return obj.obj_new_string_copy(@intFromPtr("8.6".ptr), 3);
}

pub fn info_patchlevel() i32 {
    return obj.obj_new_string_copy(@intFromPtr("8.6.15".ptr), 6);
}

pub fn info_hostname() i32 {
    return obj.obj_new_string_copy(@intFromPtr("wasm".ptr), 4);
}

/// ``info level`` — return the current call-frame depth as an
/// integer TclObj.  Matches Tcl 9's ``InfoLevelCmd`` for the
/// zero-arg form (``Tcl_NewWideIntObj((int)varFramePtr->level)``).
///
/// The single-arg form ``info level N`` would return the argv
/// list that entered frame ``N``.  Our runtime doesn't retain
/// per-frame argv today, but ``info level 0`` is a mandatory
/// tcltest shape: every one of tcltest's "accessor / setter"
/// wrappers uses ``[llength [info level 0]] == 1`` as a
/// "was-I-called-with-no-args?" check (e.g. ``workingDirectory``,
/// ``outputChannel``, ``temporaryDirectory``, the per-option
/// accessor procs generated by ``Option``).  Trapping with
/// ``bad level "0"`` pokes a hole in every preamble path that
/// routes through those procs.
///
/// For ``info level 0`` we return a single-element placeholder
/// list so the common ``llength [info level 0] == 1`` branch
/// fires — that's the "called without args, return current value"
/// path tcltest wants during initialisation and configure reads.
/// The element text is informational only; callers inspect the
/// list's *length*, not its contents.  The single-arg form for
/// non-zero ``N`` still raises ``bad level "N"`` — that would
/// need real per-frame argv tracking.
pub fn info_level(arg: i32) i32 {
    // ``info level`` (no arg) — return the integer frame depth,
    // matching Tcl 9's ``InfoLevelCmd`` zero-arg form.  The
    // dispatcher passes ``arg == 0`` (null TclObj) when the
    // caller wrote ``info level`` without a following word.
    if (arg == 0) {
        return obj_new_int(frames.frame_get_depth());
    }

    // Parse the arg as a (possibly signed) integer.  Tcl accepts
    // both positive levels (``info level 1`` = absolute frame 1)
    // and negative levels (``info level -1`` = caller, one step
    // up).  Under full fidelity we'd distinguish these — for the
    // moment we accept the literal ``0`` form that tcltest relies
    // on, plus negative offsets that walk up the stack.  Positive
    // absolute levels without per-frame argv-is-stored-by-level
    // mapping still raise ``bad level``.
    const arg_s = obj_ensure_string(arg);
    const parsed_offset = parse_signed_int(arg_s.ptr, arg_s.len);
    if (parsed_offset.ok) {
        const n = parsed_offset.value;
        if (n == 0) {
            // ``info level 0`` outside any proc has no
            // invocation — Tcl 9 returns ``bad level "0"``.
            if (frames.frame_get_depth() == 0) {
                const catch_mod = @import("../interp/tcl_catch.zig");
                const literal = "bad level \"0\"";
                const msg = obj_new_string(
                    @intCast(@intFromPtr(literal.ptr)),
                    @intCast(literal.len),
                );
                catch_mod.tcl_cmd_error(msg);
                return obj_new_string(0, 0);
            }
            // Return the real argv list recorded by the caller
            // (``eval_proc_call_bucket`` for interpreted procs,
            // or the compiled-proc prologue's ``frame_set_argv``
            // call for compiled procs).  If no argv was recorded
            // (legacy callers before the instrumentation, top-level
            // frames, etc.) fall back to a 1-element placeholder
            // so tcltest's ``[llength [info level 0]] == 1``
            // check still fires.
            const argv = frames.frame_get_argv(0);
            if (argv != 0) return argv;
            return obj_new_string(
                @intCast(@intFromPtr("info_level_0".ptr)),
                12,
            );
        }
        if (n < 0) {
            // Negative levels are relative — ``-1`` = caller,
            // ``-2`` = caller's caller, etc.  ``frame_get_argv``
            // expects a non-negative offset, so flip the sign.
            const offset: i32 = -n;
            const depth_i: i32 = frames.frame_get_depth();
            if (offset >= depth_i) {
                // Walking off the top of the frame stack —
                // raise ``bad level "N"`` to match Tcl.
                return raise_bad_level(arg_s);
            }
            const argv = frames.frame_get_argv(offset);
            if (argv != 0) return argv;
            return obj_new_string(0, 0);
        }
        // Positive absolute level.  We don't yet map absolute
        // levels to per-frame argv (would need a level-indexed
        // table), so translate to a relative offset from the
        // current depth and reuse ``frame_get_argv``.
        const depth_i: i32 = frames.frame_get_depth();
        if (n <= 0 or n > depth_i) {
            return raise_bad_level(arg_s);
        }
        const offset: i32 = depth_i - n;
        const argv = frames.frame_get_argv(offset);
        if (argv != 0) return argv;
        return obj_new_string(0, 0);
    }

    // Arg isn't parseable as an integer — raise ``bad level "X"``.
    return raise_bad_level(arg_s);
}

/// Parse a signed integer from a byte span.  Returns ``ok = false``
/// on empty input, a non-digit after an optional leading sign, or
/// an integer that would overflow an i32 (the caller treats
/// ``ok = false`` as "not a number" and surfaces ``bad level``,
/// which matches what tclsh does for out-of-range arguments).
const ParsedInt = struct { value: i32, ok: bool };
fn parse_signed_int(ptr: u32, len: u32) ParsedInt {
    if (len == 0) return .{ .value = 0, .ok = false };
    const p: [*]const u8 = @ptrFromInt(ptr);
    var i: u32 = 0;
    var negative: bool = false;
    if (p[0] == '-') {
        negative = true;
        i = 1;
    } else if (p[0] == '+') {
        i = 1;
    }
    if (i >= len) return .{ .value = 0, .ok = false };
    // Accumulate into an i64 so the overflow check at the end
    // reliably catches values outside the i32 range — doing the
    // multiply in i32 would itself wrap before we could test.
    var v: i64 = 0;
    while (i < len) : (i += 1) {
        const c = p[i];
        if (c < '0' or c > '9') return .{ .value = 0, .ok = false };
        v = v * 10 + @as(i64, @intCast(c - '0'));
        // Bail the moment the magnitude exceeds i32's positive
        // range + 1 — that covers both ``+2147483648`` and any
        // larger input without having to special-case
        // ``-2147483648``.
        if (v > 2147483648) return .{ .value = 0, .ok = false };
    }
    if (negative) v = -v;
    if (v > 2147483647 or v < -2147483648) return .{ .value = 0, .ok = false };
    return .{ .value = @intCast(v), .ok = true };
}

fn raise_bad_level(arg_s: anytype) i32 {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const prefix = "bad level \"";
    const suffix = "\"";
    const total: u32 = @as(u32, @intCast(prefix.len)) + arg_s.len + @as(u32, @intCast(suffix.len));
    const buf = alloc(total);
    const d: [*]u8 = @ptrFromInt(buf);
    for (prefix, 0..) |b, k| d[k] = b;
    if (arg_s.len > 0) {
        const sp: [*]const u8 = @ptrFromInt(arg_s.ptr);
        for (0..arg_s.len) |k| d[prefix.len + k] = sp[k];
    }
    for (suffix, 0..) |b, k| d[prefix.len + arg_s.len + k] = b;
    const msg = obj_new_string(@bitCast(buf), @bitCast(total));
    catch_mod.tcl_cmd_error(msg);
    return obj_new_string(0, 0);
}

/// ``info script`` — return the source file of the script
/// currently being evaluated.  Our WASM runtime is a single
/// compiled unit with no real filesystem sourcing, so this
/// always returns the empty string.  Matches Tcl 9's behaviour
/// when not sourcing a file (``Tcl_SetObjResult(interp,
/// Tcl_NewObj())``).  Consumers that use ``info script`` to
/// Phase 8: ``info frame ?N?`` — call-site metadata for the frame
/// stack.  Without an argument returns the current frame depth as
/// an integer (same as ``info level``'s no-arg form, but counting
/// every frame including the top-level).  With an argument returns
/// a Tcl dict-list ``{type X cmd Y proc Z line N script S}``
/// describing the requested frame.
///
/// Real Tcl uses 1-based depth where frame 1 is the outermost
/// (top-level eval) and frame N is the innermost (current proc).
/// We mirror that.
///
/// Note: this is the minimum-viable implementation.  Reference
/// Tcl populates additional fields like ``-source`` (file path),
/// ``-lambda`` (apply's body), and ``-procs`` from the actual
/// invocation site.  The frame metadata captured at ``frame_push``
/// time is what we expose; richer source-position tracking
/// (per-command line numbers) is a follow-up that needs the
/// parser to thread line info through to the dispatcher.
pub fn info_frame(arg: i32) i32 {
    // No arg: return the depth (integer).  Real Tcl includes the
    // synthetic frame 0 for top-level; we count from 1 (matching
    // ``info level``'s convention).
    if (arg == 0) {
        const d: i64 = @intCast(frames.frame_get_depth());
        return obj_new_int(d + 1);
    }
    // With arg: parse as integer depth.  Tcl uses 1-based depth
    // where frame 1 is the synthetic top-level script frame and
    // frame N is the innermost pushed frame.  Our internal
    // ``frame_depth`` counts only PUSHED frames (top-level = 0
    // pushed), so the conversion is ``internal_abs = N - 1``.
    // Negative offsets count from the current frame
    // (``info frame -1`` = caller's frame).
    const want = obj.obj_get_int(arg);
    const cur_depth: i64 = @intCast(frames.frame_get_depth());
    const tcl_max: i64 = cur_depth + 1; // top-level + every push
    const tcl_abs: i64 = if (want > 0) want else tcl_max + want;
    if (tcl_abs < 1 or tcl_abs > tcl_max) {
        // Reference Tcl 9 raises ``bad level "<N>"`` here — see
        // ``InfoFrameCmd`` in ``tclCmdIL.c``.  Routing through the
        // standard error path lets ``catch {info frame ...}`` see
        // a real ``catch`` outcome (rc=1) instead of a fabricated
        // ``type eval`` dict.  Codex review.
        const catch_mod = @import("../interp/tcl_catch.zig");
        const prefix: []const u8 = "bad level \"";
        const arg_s = obj.obj_ensure_string(arg);
        const total: u32 = @intCast(prefix.len + arg_s.len + 1);
        const buf = obj.alloc(total);
        if (buf == 0) return obj_new_string(0, 0);
        const dst: [*]u8 = @ptrFromInt(buf);
        var off: u32 = 0;
        for (prefix) |c| {
            dst[off] = c;
            off += 1;
        }
        if (arg_s.len > 0) {
            const ap: [*]const u8 = @ptrFromInt(arg_s.ptr);
            for (0..arg_s.len) |i| {
                dst[off] = ap[i];
                off += 1;
            }
        }
        dst[off] = '"';
        const msg = obj.obj_new_string_take(buf, total, total);
        catch_mod.tcl_cmd_error(msg);
        return obj_new_string(0, 0);
    }
    // Frame 1 is the synthetic top-level (no FrameInfo).
    if (tcl_abs == 1) {
        return obj.obj_new_string_copy(@intFromPtr("type eval".ptr), 9);
    }
    const fi = frames.frame_get_info(@intCast(tcl_abs - 1)) orelse {
        return obj.obj_new_string_copy(@intFromPtr("type eval".ptr), 9);
    };
    // Build the dict-list incrementally.  We use string concat
    // rather than dict_set because ``info frame``'s output is
    // observed as a list-of-pairs by most callers; reference Tcl
    // emits it the same way.
    const list_mod = @import("../valtypes/tcl_list.zig");
    var result = obj_new_string(0, 0);
    // -type
    const type_str: []const u8 = switch (fi.type) {
        .PROC => "proc",
        .SOURCE => "source",
        .EVAL => "eval",
        .UPLEVEL => "uplevel",
        .ALIAS => "alias",
        .UNKNOWN => "eval",
    };
    {
        const k = obj.obj_new_string_copy(@intFromPtr("type".ptr), 4);
        const v = obj.obj_new_string_copy(@intFromPtr(type_str.ptr), @intCast(type_str.len));
        const r1 = list_mod.tcl_list(result, k);
        obj.tcl_obj_release(result);
        const r2 = list_mod.tcl_list(r1, v);
        obj.tcl_obj_release(r1);
        obj.tcl_obj_release(k);
        obj.tcl_obj_release(v);
        result = r2;
    }
    // -line N
    if (fi.line != 0) {
        const k = obj.obj_new_string_copy(@intFromPtr("line".ptr), 4);
        const v = obj_new_int(@intCast(fi.line));
        const r1 = list_mod.tcl_list(result, k);
        obj.tcl_obj_release(result);
        const r2 = list_mod.tcl_list(r1, v);
        obj.tcl_obj_release(r1);
        obj.tcl_obj_release(k);
        obj.tcl_obj_release(v);
        result = r2;
    }
    // -proc NAME (PROC frames only)
    if (fi.type == .PROC and fi.proc_name != 0) {
        const k = obj.obj_new_string_copy(@intFromPtr("proc".ptr), 4);
        const r1 = list_mod.tcl_list(result, k);
        obj.tcl_obj_release(result);
        const r2 = list_mod.tcl_list(r1, fi.proc_name);
        obj.tcl_obj_release(r1);
        obj.tcl_obj_release(k);
        result = r2;
    }
    // -cmd TEXT
    if (fi.cmd_text != 0) {
        const k = obj.obj_new_string_copy(@intFromPtr("cmd".ptr), 3);
        const r1 = list_mod.tcl_list(result, k);
        obj.tcl_obj_release(result);
        const r2 = list_mod.tcl_list(r1, fi.cmd_text);
        obj.tcl_obj_release(r1);
        obj.tcl_obj_release(k);
        result = r2;
    }
    // -script BODY (Phase 8 follow-up).  The frame's script_obj is
    // stamped by the proc-call dispatcher (interpreted procs) and
    // by the compiled-proc prologue (Phase 8 follow-up codegen
    // change), so this field is populated for every proc frame
    // whose body source is known at compile time.
    if (fi.script_obj != 0) {
        const k = obj.obj_new_string_copy(@intFromPtr("script".ptr), 6);
        const r1 = list_mod.tcl_list(result, k);
        obj.tcl_obj_release(result);
        const r2 = list_mod.tcl_list(r1, fi.script_obj);
        obj.tcl_obj_release(r1);
        obj.tcl_obj_release(k);
        result = r2;
    }
    return result;
}

/// resolve path-relative includes (``source [file join [info
/// script] ...]``) must treat the empty result as "no location
/// available" — tcllib's ``testutilities.tcl`` already does
/// (hence our stub replaces its ``source`` block).
pub fn info_script() i32 {
    return obj_new_string(0, 0);
}

// -- ``info commands`` / ``info procs`` walker ----------------------------

/// Kinds of entry the cmd-table walker emits.  ``commands`` yields
/// every live Command (including aliases and import redirects);
/// ``procs`` narrows to interpreted procs only.
const WalkKind = enum { commands, procs };

/// Context carried through both the sizing + filling passes.  Using
/// two passes keeps the output buffer tightly sized — the bump
/// allocator can't realloc, so we can't grow in place.
const CmdWalkCtx = struct {
    kind: WalkKind,
    /// Non-null when a pattern was supplied.  Matched against the
    /// simple (unqualified) name of each candidate.
    pat_ptr: u32,
    pat_len: u32,
    has_pattern: bool,
    /// True when the user-supplied pattern was namespace-qualified
    /// (``::ns::pat``).  Emit results as FQ names in that case;
    /// for unqualified patterns we emit the simple name so that
    /// ``info commands foo`` returns ``foo`` rather than ``::foo``
    /// (info-4.3 / 4.4).
    qualified_pattern: bool,
    /// Cumulative length of the fully-qualified names we'll emit,
    /// plus one separator byte per gap.  Populated during pass 1.
    total: u32,
    /// Running count of emitted entries.  Used for the space-before-
    /// each-after-first separator pattern.
    count: u32,
    /// Only populated during pass 2.
    buf: u32,
    off: u32,
};

/// Predicate: does this Command match the current filter kind and
/// pattern?  Called once per live bucket in both the sizing and
/// filling passes; encapsulates the per-bucket filtering logic so
/// the two walkers stay in lockstep.
fn entry_matches(ctx: *const CmdWalkCtx, name_ptr: u32, name_len: u32, cmd: u32) bool {
    if (cmd == 0) return false;
    const flags: u32 = @bitCast(read_i32(cmd + procs.OFF_FLAGS));
    // Dead import redirect (``namespace forget``).  Live imports
    // have ``ImportedCmdData.real_cmd != 0``; forgotten ones zero
    // that slot.  Skip both kinds so stale entries disappear from
    // ``info commands``.
    if ((flags & procs.CMD_IMPORTED) != 0) {
        const desc: u32 = @bitCast(read_i32(cmd + procs.OFF_PARAMS_OBJ));
        if (desc == 0) return false;
        const real: u32 = @bitCast(read_i32(desc));
        if (real == 0) return false;
    }
    switch (ctx.kind) {
        .commands => {},
        .procs => {
            // ``info procs`` only reports interpreted procs —
            // compiled, alias, and import redirects don't qualify.
            if ((flags & procs.CMD_ALIAS) != 0) return false;
            if ((flags & procs.CMD_IMPORTED) != 0) return false;
            const func_idx = read_i32(cmd + procs.OFF_FUNC_IDX);
            if (func_idx != 0) return false;
            const body = read_i32(cmd + 16); // OFF_BODY_OBJ
            if (body == 0) return false;
        },
    }
    if (ctx.has_pattern) {
        if (!tcl_string.glob_match(ctx.pat_ptr, ctx.pat_len, name_ptr, name_len)) {
            return false;
        }
    }
    return true;
}

/// Compute the length of a Command's fully-qualified name — we read
/// it straight out of the Command's stored name slot.  ``rename`` /
/// ``expose`` keep this slot in sync with the live identity.
fn cmd_stored_name(cmd: u32) struct { ptr: u32, len: u32 } {
    const p: u32 = @bitCast(read_i32(cmd));
    const l: u32 = @bitCast(read_i32(cmd + 4));
    return .{ .ptr = p, .len = l };
}

/// Probe one namespace's ``cmd_table`` for matching entries.  If
/// ``ctx.buf == 0`` we're in the sizing pass and just increment
/// ``ctx.total`` / ``ctx.count``; otherwise we append each FQN into
/// ``ctx.buf`` separated by single spaces.
fn scan_ns_cmd_table(ns_addr: u32, ctx: *CmdWalkCtx) void {
    const ns: *const tcl_ns.Namespace = @ptrFromInt(ns_addr);
    if (ns.cmd_table.buf == 0) return;
    // For non-root namespaces we want each emitted name to be FQ
    // (``::ns::cmd``) — both for native procs whose stored name
    // is the FQN and for imports whose redirect stores only the
    // simple name.  Resolve the namespace's FQN once up front so
    // ``emit_name_in_ns`` can synthesise it cheaply.
    const ns_full = tcl_ns.ns_full_name(ns_addr);
    var i: u32 = 0;
    while (i < ns.cmd_table.cap) : (i += 1) {
        const bucket = ns.cmd_table.buf + i * BUCKET_SIZE;
        const name_ptr: u32 = @bitCast(read_i32(bucket));
        if (name_ptr == 0) continue;
        const name_len: u32 = @bitCast(read_i32(bucket + 4));
        const cmd: u32 = @bitCast(read_i32(bucket + tcl_ns.OFF_HANDLE));
        if (!entry_matches(ctx, name_ptr, name_len, cmd)) continue;
        const fqn = cmd_stored_name(cmd);
        if (fqn.len == 0) continue;
        // Tcl 9 ``InfoCommandsCmd`` emits FQ names only when the
        // user-supplied pattern was namespace-qualified; bare
        // ``info commands foo`` returns ``foo`` (info-4.3).  When
        // we must emit FQ, prefer the stored name if it's already
        // qualified, else synthesise from the *bucket* name (the
        // dest-ns simple name for an import redirect).
        const fp: [*]const u8 = @ptrFromInt(fqn.ptr);
        const stored_is_qualified = fqn.len >= 2 and fp[0] == ':' and fp[1] == ':';
        if (ctx.qualified_pattern) {
            if (stored_is_qualified) {
                emit_name(ctx, fqn.ptr, fqn.len);
            } else {
                emit_name_in_ns(ctx, ns_full.ptr, ns_full.len, name_ptr, name_len);
            }
        } else {
            // Unqualified pattern → simple name in the result.
            emit_name(ctx, name_ptr, name_len);
        }
    }
}

/// Emit ``<ns_full>::<simple>`` in one go — sizing-pass + fill-pass
/// shared with :fn:`emit_name`.  Used for imports whose redirect
/// stores only the simple name.  When *ns_full* is the root
/// namespace (``::``) we emit just ``::<simple>``.
fn emit_name_in_ns(
    ctx: *CmdWalkCtx,
    ns_ptr: u32,
    ns_len: u32,
    simple_ptr: u32,
    simple_len: u32,
) void {
    // ``ns_full`` for root is ``"::"`` (len=2) — emit
    // ``::simple``.  For non-root ``::a::b`` (no trailing ``::``),
    // emit ``::a::b::simple``.
    const need_sep: bool = ns_len > 2;
    const total_name: u32 = ns_len + (if (need_sep) @as(u32, 2) else 0) + simple_len;
    if (ctx.buf == 0) {
        if (ctx.count > 0) ctx.total += 1;
        ctx.total += total_name;
        ctx.count += 1;
        return;
    }
    if (ctx.count > 0) {
        const d: [*]u8 = @ptrFromInt(ctx.buf + ctx.off);
        d[0] = ' ';
        ctx.off += 1;
    }
    const dst: [*]u8 = @ptrFromInt(ctx.buf + ctx.off);
    const np: [*]const u8 = @ptrFromInt(ns_ptr);
    for (0..ns_len) |k| dst[k] = np[k];
    var off2: u32 = ns_len;
    if (need_sep) {
        dst[off2] = ':';
        dst[off2 + 1] = ':';
        off2 += 2;
    }
    const sp: [*]const u8 = @ptrFromInt(simple_ptr);
    for (0..simple_len) |k| dst[off2 + k] = sp[k];
    ctx.off += total_name;
    ctx.count += 1;
}

/// Emit one FQN — sizing pass if ``ctx.buf == 0``, filling pass
/// otherwise.  The separator is a single space between entries.
fn emit_name(ctx: *CmdWalkCtx, name_ptr: u32, name_len: u32) void {
    if (ctx.buf == 0) {
        if (ctx.count > 0) ctx.total += 1;
        ctx.total += name_len;
        ctx.count += 1;
        return;
    }
    if (ctx.count > 0) {
        const d: [*]u8 = @ptrFromInt(ctx.buf + ctx.off);
        d[0] = ' ';
        ctx.off += 1;
    }
    memcpy(ctx.buf + ctx.off, name_ptr, name_len);
    ctx.off += name_len;
    ctx.count += 1;
}

/// Walk the current ns's resolution chain (context + namespace path
/// + root) and emit every matching Command.  Mirrors
/// ``InfoCommandsCmd`` in ``tclCmdIL.c`` for the unqualified case.
///
/// De-duplication: we visit root last, and the context-ns probe
/// already covers ``root == current_ns`` directly, so a root-level
/// command reachable from the root ns appears once rather than
/// twice.  Path entries pointing back at the context are skipped
/// (mirrors ``ns_find_command``).
fn walk_unqualified_path(ctx: *CmdWalkCtx) void {
    const root = tcl_ns.ns_root();
    const start: u32 = tcl_ns.ns_current();

    scan_ns_cmd_table(start, ctx);

    // Walk ``namespace path`` entries.  Track whether the path
    // already includes root so the trailing "always scan root"
    // fallback doesn't double-emit root entries when a caller
    // explicitly lists ``::`` in their path.
    const plen = tcl_ns.ns_path_len(start);
    var root_in_path: bool = false;
    var pi: u32 = 0;
    while (pi < plen) : (pi += 1) {
        const e = tcl_ns.ns_path_entry(start, pi);
        if (e.target_ns == 0 or e.target_ns == start) continue;
        if (e.target_ns == root) {
            root_in_path = true;
        }
        scan_ns_cmd_table(e.target_ns, ctx);
    }

    if (start != root and !root_in_path) scan_ns_cmd_table(root, ctx);

    // Walk the builtin command table.  Builtins (``puts``, ``string``,
    // ``try``, ``trace``, ...) live in a static array assembled from
    // each ``cmds/<name>.zig`` rather than in any namespace's
    // ``cmd_table``, so without this pass ``info commands t*`` /
    // ``info commands string`` etc. came back empty.  ``info procs``
    // narrows to interpreted procs only, so it skips this pass.
    if (ctx.kind == .commands) scan_builtin_table(ctx);
}

/// Walk the static BUILTINS slice from ``tcl_cmd_table.zig`` and emit
/// each builtin command name that matches the filter.  Names are
/// emitted as-is (unqualified) — the builtins live in the root ns
/// for resolution purposes but we don't currently auto-prefix
/// ``::`` to ``info commands`` results to match tclsh's flat output.
fn scan_builtin_table(ctx: *CmdWalkCtx) void {
    const cmd_table = @import("../dispatch/tcl_cmd_table.zig");
    const entries = cmd_table.entries();
    for (entries) |e| {
        const name_ptr: u32 = @intFromPtr(e.name.ptr);
        const name_len: u32 = @intCast(e.name.len);
        if (ctx.has_pattern) {
            if (!tcl_string.glob_match(ctx.pat_ptr, ctx.pat_len, name_ptr, name_len)) {
                continue;
            }
        }
        if (ctx.buf == 0) {
            if (ctx.count > 0) ctx.total += 1; // separator
            ctx.total += name_len;
            ctx.count += 1;
        } else {
            if (ctx.count > 0) {
                const sep: [*]u8 = @ptrFromInt(ctx.buf + ctx.off);
                sep[0] = ' ';
                ctx.off += 1;
            }
            const dst: [*]u8 = @ptrFromInt(ctx.buf + ctx.off);
            const src: [*]const u8 = @ptrFromInt(name_ptr);
            for (0..name_len) |k| dst[k] = src[k];
            ctx.off += name_len;
            ctx.count += 1;
        }
    }
}

/// Walk only the target namespace's ``cmd_table`` — used when the
/// caller supplied a qualified pattern (``::foo::*``).  The
/// pattern's trailing simple component is matched against each
/// name; the leading namespace component has already been stripped
/// by the qualified-pattern parser.
fn walk_qualified(target_ns: u32, ctx: *CmdWalkCtx) void {
    if (target_ns == 0) return;
    scan_ns_cmd_table(target_ns, ctx);
    // Builtins (``apply``, ``puts``, ``string``, …) live in a
    // static table outside any namespace's ``cmd_table`` but are
    // resolution-attached to the root namespace.  Without this
    // pass, ``info commands ::apply`` came back empty and
    // apply.test bailed out at its early ``if {[info commands
    // ::apply] eq {}} { return }`` guard, producing no tcltest
    // summary at all.  Mirrors the same opt-in in
    // :func:`walk_unqualified_path` for ``commands``-kind walks.
    if (ctx.kind == .commands and target_ns == tcl_ns.ns_root()) {
        scan_builtin_table(ctx);
    }
}

/// Produce a TclObj list of matching FQNs, space-separated.
/// ``pattern`` may be empty (no filter), unqualified (match
/// against simple names in the current-ns walk), or qualified
/// (split into target-ns + simple-pattern via
/// ``ns_resolve_qualified``).
fn run_cmd_walk(kind: WalkKind, pattern: i32) i32 {
    var ctx: CmdWalkCtx = .{
        .kind = kind,
        .pat_ptr = 0,
        .pat_len = 0,
        .has_pattern = false,
        .qualified_pattern = false,
        .total = 0,
        .count = 0,
        .buf = 0,
        .off = 0,
    };

    var qualified_target: u32 = 0;
    if (pattern != 0) {
        const p = obj_ensure_string(pattern);
        if (p.len > 0) {
            ctx.has_pattern = true;
            ctx.pat_ptr = p.ptr;
            ctx.pat_len = p.len;
            // Detect qualified pattern: any ``::`` substring.
            var has_colons = false;
            const src: [*]const u8 = @ptrFromInt(p.ptr);
            if (p.len >= 2) {
                var i: u32 = 0;
                while (i + 1 < p.len) : (i += 1) {
                    if (src[i] == ':' and src[i + 1] == ':') {
                        has_colons = true;
                        break;
                    }
                }
            }
            if (has_colons) {
                const r = tcl_ns.ns_resolve_qualified(tcl_ns.ns_current(), p.ptr, p.len);
                qualified_target = r.target_ns;
                ctx.pat_ptr = r.simple_ptr;
                ctx.pat_len = r.simple_len;
                ctx.qualified_pattern = true;
                if (ctx.pat_len == 0) {
                    // Pattern was ``::`` alone or trailing ``::`` —
                    // matches nothing.
                    return obj_new_string(0, 0);
                }
                // BUILTIN-tier namespaces (``::tcl::mathop::*``,
                // ``::tcl::mathfunc::*``, …) aren't represented in
                // the namespace tree until something pokes them, so
                // a fresh ``info commands ::tcl::mathop::*`` would
                // come back empty.  Materialise unconditionally
                // (not gated on ``qualified_target == 0``): the user
                // may already have created the ns with
                // ``namespace eval ::tcl::mathop {}``, in which case
                // the resolver would return non-zero but the
                // ``cmd_table`` would still be empty.  ``materialise``
                // is idempotent on already-populated forwards and
                // a no-op for non-BUILTINS prefixes, so the override
                // only fires when there's something useful to
                // populate.
                if (ctx.kind == .commands) {
                    const builtin_ns = @import("../dispatch/tcl_builtin_ns.zig");
                    // Reconstruct the namespace prefix: everything
                    // up to (and not including) the last ``::``
                    // before the trailing simple pattern.
                    var last_sep: i32 = -1;
                    var k: u32 = 0;
                    while (k + 1 < p.len) : (k += 1) {
                        if (src[k] == ':' and src[k + 1] == ':') {
                            last_sep = @intCast(k);
                            k += 1;
                        }
                    }
                    if (last_sep > 0) {
                        const prefix_len: u32 = @intCast(last_sep);
                        const materialised = builtin_ns.materialise(p.ptr, prefix_len);
                        if (materialised != 0) qualified_target = materialised;
                    }
                }
            }
        }
    }

    // Pass 1: size.
    if (qualified_target != 0) {
        walk_qualified(qualified_target, &ctx);
    } else {
        walk_unqualified_path(&ctx);
    }
    if (ctx.total == 0) return obj_new_string(0, 0);

    // Pass 2: fill.
    ctx.buf = alloc(ctx.total);
    ctx.off = 0;
    ctx.count = 0;
    if (qualified_target != 0) {
        walk_qualified(qualified_target, &ctx);
    } else {
        walk_unqualified_path(&ctx);
    }
    return obj_new_string(@bitCast(ctx.buf), @bitCast(ctx.off));
}

/// info commands ?pattern? — public entry.  ``pattern == 0`` means
/// "no pattern supplied"; a non-zero TclObj is treated as the
/// filter.
pub fn info_commands(pattern: i32) i32 {
    return run_cmd_walk(.commands, pattern);
}

/// info procs ?pattern? — same walk, interpreted-procs-only filter.
pub fn info_procs(pattern: i32) i32 {
    return run_cmd_walk(.procs, pattern);
}

// -- ``info default`` ------------------------------------------------------

/// Raise ``procedure "X" doesn't have an argument "Y"`` — the Tcl 9
/// error string for ``info default`` when the arg name isn't in the
/// proc's params list.
fn raise_missing_argument(proc_ptr: u32, proc_len: u32, arg_ptr: u32, arg_len: u32) void {
    const catch_mod = @import("../interp/tcl_catch.zig");
    const prefix = "procedure \"";
    const middle = "\" doesn't have an argument \"";
    const suffix = "\"";
    const total: u32 = @as(u32, @intCast(prefix.len)) + proc_len +
        @as(u32, @intCast(middle.len)) + arg_len +
        @as(u32, @intCast(suffix.len));
    const buf = alloc(total);
    const d: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix, 0..) |b, k| d[k] = b;
    off = @intCast(prefix.len);
    if (proc_len > 0) {
        const sp: [*]const u8 = @ptrFromInt(proc_ptr);
        for (0..proc_len) |k| d[off + k] = sp[k];
    }
    off += proc_len;
    for (middle, 0..) |b, k| d[off + k] = b;
    off += @as(u32, @intCast(middle.len));
    if (arg_len > 0) {
        const sp: [*]const u8 = @ptrFromInt(arg_ptr);
        for (0..arg_len) |k| d[off + k] = sp[k];
    }
    off += arg_len;
    for (suffix, 0..) |b, k| d[off + k] = b;
    const msg = obj_new_string(@bitCast(buf), @bitCast(total));
    catch_mod.tcl_cmd_error(msg);
}

/// info default proc arg varName — write the default value of
/// parameter ``arg`` in proc ``proc`` into the caller's variable
/// ``varName`` and return 1 when a default is present.  When the
/// arg has no default, set ``varName`` to the empty string and
/// return 0.  When ``arg`` isn't in the proc's params list, raise
/// ``procedure "X" doesn't have an argument "Y"`` (tclsh parity).
/// Non-interpreted-proc resolution failures are handled in
/// ``resolve_interpreted_proc``.
pub export fn info_default(proc_name: i32, arg_name: i32, var_name: i32) i32 {
    const cmd = resolve_interpreted_proc(proc_name);
    if (cmd == 0) return obj_new_int(0);
    const params_obj = procs.proc_get_params(@bitCast(cmd));
    const arg_s = obj_ensure_string(arg_name);
    const proc_s = obj_ensure_string(proc_name);
    if (params_obj == 0) {
        raise_missing_argument(proc_s.ptr, proc_s.len, arg_s.ptr, arg_s.len);
        return obj_new_int(0);
    }
    const params_s = obj_ensure_string(params_obj);

    const count = obj.list_count_elements(params_s.ptr, params_s.len);
    var i: i64 = 0;
    while (i < count) : (i += 1) {
        const elt = obj.list_element_at(params_s.ptr, params_s.len, i);
        // elt.start is an offset from params_s.ptr, not an absolute address.
        const elt_ptr = params_s.ptr + elt.start;
        // Is this a two-element ``{name default}`` pair?  Split the
        // element into its own sub-elements and compare.
        const sub_count = obj.list_count_elements(elt_ptr, elt.len);
        if (sub_count == 0) continue;
        const name_sub = obj.list_element_at(elt_ptr, elt.len, 0);
        if (name_sub.len != arg_s.len) continue;
        if (name_sub.len == 0) continue;
        const ap: [*]const u8 = @ptrFromInt(arg_s.ptr);
        const np: [*]const u8 = @ptrFromInt(elt_ptr + name_sub.start);
        var match = true;
        var k: u32 = 0;
        while (k < arg_s.len) : (k += 1) {
            if (ap[k] != np[k]) {
                match = false;
                break;
            }
        }
        if (!match) continue;
        // Found the parameter.  If it has a default, pass that
        // through ``var_name``; otherwise set ``var_name`` to empty
        // (Tcl 9 InfoDefaultCmd sets to a fresh empty TclObj so the
        // caller's variable is always materialised, unlike the
        // pre-wave behaviour of leaving it unset).
        if (sub_count < 2) {
            _ = frames.var_set(var_name, obj_new_string(0, 0));
            return obj_new_int(0);
        }
        const default_sub = obj.list_element_at(elt_ptr, elt.len, 1);
        const default_obj = obj_new_string(@bitCast(elt_ptr + default_sub.start), @bitCast(default_sub.len));
        _ = frames.var_set(var_name, default_obj);
        return obj_new_int(1);
    }
    // Arg isn't in the proc's params — match tclsh's error shape.
    raise_missing_argument(proc_s.ptr, proc_s.len, arg_s.ptr, arg_s.len);
    return obj_new_int(0);
}

// -- ``info vars`` ---------------------------------------------------------

/// ``info vars ?pattern?`` — return a list of variable names visible
/// in the current scope that match the optional glob ``pattern``.
///
/// Qualified patterns (containing ``::``) scan the root namespace's
/// var_table (scalars) AND the array directory (Tcl arrays), since the
/// two stores are disjoint.  Unqualified patterns scan the current
/// call frame's locals; if no frame is active, scans the global scope.
///
/// This is used by tcllib's ``counter::names``:
///   ``foreach v [info vars ::counter::T-*] { … }``
/// which expects fully-qualified names like ``::counter::T-simple``.
pub fn info_vars(pattern: i32) i32 {
    const tcl_array = @import("../valtypes/tcl_array.zig");

    var pat_ptr: u32 = 0;
    var pat_len: u32 = 0;
    var has_pattern = false;
    var is_qualified = false;

    if (pattern != 0) {
        const ps = obj_ensure_string(pattern);
        if (ps.len > 0) {
            has_pattern = true;
            pat_ptr = ps.ptr;
            pat_len = ps.len;
            const pp: [*]const u8 = @ptrFromInt(ps.ptr);
            var k: u32 = 0;
            while (k + 1 < ps.len) : (k += 1) {
                if (pp[k] == ':' and pp[k + 1] == ':') {
                    is_qualified = true;
                    break;
                }
            }
        }
    }

    if (is_qualified) {
        // Vars created via ``namespace eval X { variable Y }`` are
        // stored flat-keyed in root's var_table as ``X::Y`` (see
        // :func:`scope.eval_variable`).  Tcl's ``info vars
        // pat::*`` should:
        //   * accept the pattern with or without a leading ``::``
        //     (both ``ns::*`` and ``::ns::*`` work);
        //   * match against the flat key (which has no leading
        //     ``::``);
        //   * emit results in fully-qualified form (``::ns::var``).
        // We strip a leading ``::`` from the pattern for matching
        // purposes, then prepend ``::`` on emit.
        var match_pat_ptr: u32 = pat_ptr;
        var match_pat_len: u32 = pat_len;
        if (pat_len >= 2) {
            const pp_check: [*]const u8 = @ptrFromInt(pat_ptr);
            if (pp_check[0] == ':' and pp_check[1] == ':') {
                match_pat_ptr = pat_ptr + 2;
                match_pat_len = pat_len - 2;
            }
        }

        const arr_list = tcl_array.array_dir_names_matching(pat_ptr, pat_len);
        const arr_s = obj_ensure_string(arr_list);

        const root = tcl_ns.ns_root();
        const ns: *const tcl_ns.Namespace = @ptrFromInt(root);

        var scalar_total: u32 = 0;
        var scalar_count: u32 = 0;
        if (ns.var_table.buf != 0) {
            var i: u32 = 0;
            while (i < ns.var_table.cap) : (i += 1) {
                const bucket = ns.var_table.buf + i * tcl_ns.NS_BUCKET_SIZE;
                const name_ptr: u32 = @bitCast(read_i32(bucket));
                if (name_ptr == 0) continue;
                const name_len: u32 = @bitCast(read_i32(bucket + 4));
                const var_addr: u32 = @bitCast(read_i32(bucket + tcl_ns.OFF_HANDLE));
                if (var_addr == 0) continue;
                // ``info vars`` lists declared variables — uninitialised
                // ones (``variable foo`` with no value) still appear,
                // unlike ``info exists`` which gates on actual storage.
                if (!tcl_string.glob_match(match_pat_ptr, match_pat_len, name_ptr, name_len)) continue;
                if (scalar_count > 0) scalar_total += 1;
                scalar_total += name_len + 2; // ``::`` prefix on emit
                scalar_count += 1;
            }
        }

        if (scalar_count == 0) return arr_list;

        const arr_sep: u32 = if (arr_s.len > 0) @as(u32, 1) else @as(u32, 0);
        const merge_total: u32 = arr_s.len + arr_sep + scalar_total;
        const merge_buf = alloc(merge_total);
        var off: u32 = 0;
        if (arr_s.len > 0) {
            memcpy(merge_buf, arr_s.ptr, arr_s.len);
            off = arr_s.len;
        }
        var written: u32 = if (arr_s.len > 0) 1 else 0;
        if (ns.var_table.buf != 0) {
            var i: u32 = 0;
            while (i < ns.var_table.cap) : (i += 1) {
                const bucket = ns.var_table.buf + i * tcl_ns.NS_BUCKET_SIZE;
                const name_ptr: u32 = @bitCast(read_i32(bucket));
                if (name_ptr == 0) continue;
                const name_len: u32 = @bitCast(read_i32(bucket + 4));
                const var_addr: u32 = @bitCast(read_i32(bucket + tcl_ns.OFF_HANDLE));
                if (var_addr == 0) continue;
                // ``info vars`` lists declared variables — uninitialised
                // ones (``variable foo`` with no value) still appear,
                // unlike ``info exists`` which gates on actual storage.
                if (!tcl_string.glob_match(match_pat_ptr, match_pat_len, name_ptr, name_len)) continue;
                if (written > 0) {
                    const d: [*]u8 = @ptrFromInt(merge_buf + off);
                    d[0] = ' ';
                    off += 1;
                }
                const dst: [*]u8 = @ptrFromInt(merge_buf + off);
                dst[0] = ':';
                dst[1] = ':';
                const np: [*]const u8 = @ptrFromInt(name_ptr);
                for (0..name_len) |k| dst[2 + k] = np[k];
                off += name_len + 2;
                written += 1;
            }
        }
        return obj_new_string(@bitCast(merge_buf), @bitCast(off));
    }

    // Unqualified pattern or no pattern.  Inside a proc frame, scan
    // the frame's hash table and collect every bucket whose stored
    // value is non-zero (Tcl semantics: ``unset`` leaves the slot's
    // bucket alive but with value 0; ``info vars`` should filter
    // those out so ``proc p1 {} { upvar 1 a x; unset x; info vars }``
    // matches the caller's surviving locals — see upvar-3.1 / 3.2).
    // Outside a proc frame, fall back to the global namespace's
    // scalar var_table.
    if (frames.frame_depth != 0) {
        return frame_locals_names_matching(pat_ptr, pat_len, has_pattern);
    }
    return root_namespace_names_matching(pat_ptr, pat_len, has_pattern);
}

/// Collector state for :func:`frame_locals_visit_current` —
/// accumulates name spans into a per-call list buffer.  The two-pass
/// shape (size pass + emit pass) keeps the output buffer right-sized
/// without resorting to a growable allocation; the visit predicate is
/// the same in both passes.
const FrameNamesCtx = struct {
    pat_ptr: u32,
    pat_len: u32,
    has_pattern: bool,
    // First pass: tally byte count + name count.
    total_bytes: u32 = 0,
    count: u32 = 0,
    // Second pass: write into ``buf`` at ``off``.
    buf: u32 = 0,
    off: u32 = 0,
    written: u32 = 0,
};

var g_names_ctx: FrameNamesCtx = .{ .pat_ptr = 0, .pat_len = 0, .has_pattern = false };

fn frame_names_should_count(name_ptr: u32, name_len: u32, value: i32) bool {
    // Skip ``unset``-cleared slots: a bucket whose value is 0 is the
    // ``unset`` marker (the bucket header stays so ``frame_pop``'s
    // mark-dirty bookkeeping can clear it later).  Aliases need their
    // target probed — an upvar to an unset variable shouldn't appear
    // in ``info vars`` either.
    if (value == 0) return false;
    if (value < 0) {
        // ALIAS_GLOBAL or ALIAS_EXT — surface only when the alias
        // chain terminates in an existing variable AND the local
        // alias name matches the requested glob pattern.  Skipping
        // the pattern check here would make ``info vars foo*`` emit
        // every live alias regardless of name (Codex P2 review).
        if (!frames.frame_alias_target_exists(value, name_ptr, name_len)) return false;
        if (g_names_ctx.has_pattern) {
            return tcl_string.glob_match(g_names_ctx.pat_ptr, g_names_ctx.pat_len, name_ptr, name_len);
        }
        return true;
    }
    if (g_names_ctx.has_pattern) {
        return tcl_string.glob_match(g_names_ctx.pat_ptr, g_names_ctx.pat_len, name_ptr, name_len);
    }
    return true;
}

fn frame_names_visit_size(_: u32, name_ptr: u32, name_len: u32, value: i32) callconv(.c) void {
    if (!frame_names_should_count(name_ptr, name_len, value)) return;
    if (g_names_ctx.has_pattern and value >= 0) {
        // size-pass already filtered via has_pattern; nothing extra
    }
    if (g_names_ctx.count > 0) g_names_ctx.total_bytes += 1; // separator
    g_names_ctx.total_bytes += name_len;
    g_names_ctx.count += 1;
}

fn frame_names_visit_emit(_: u32, name_ptr: u32, name_len: u32, value: i32) callconv(.c) void {
    if (!frame_names_should_count(name_ptr, name_len, value)) return;
    if (g_names_ctx.written > 0) {
        const dp: [*]u8 = @ptrFromInt(g_names_ctx.buf + g_names_ctx.off);
        dp[0] = ' ';
        g_names_ctx.off += 1;
    }
    memcpy(g_names_ctx.buf + g_names_ctx.off, name_ptr, name_len);
    g_names_ctx.off += name_len;
    g_names_ctx.written += 1;
}

fn frame_locals_names_matching(pat_ptr: u32, pat_len: u32, has_pattern: bool) i32 {
    g_names_ctx = .{ .pat_ptr = pat_ptr, .pat_len = pat_len, .has_pattern = has_pattern };
    frames.frame_locals_visit_current(0, &frame_names_visit_size);
    if (g_names_ctx.count == 0) return obj_new_string(0, 0);
    const total = g_names_ctx.total_bytes;
    const buf = alloc(total);
    if (buf == 0) return obj_new_string(0, 0);
    g_names_ctx.buf = buf;
    g_names_ctx.off = 0;
    g_names_ctx.written = 0;
    frames.frame_locals_visit_current(0, &frame_names_visit_emit);
    return obj.obj_new_string_take(buf, total, total);
}

/// Decide whether a top-level Var record represents an existing
/// variable for ``info vars`` purposes.  Scalars with a non-zero
/// stored value count; arrays (``VAR_ARRAY`` flag set) count too —
/// ``var_get_scalar`` returns 0 for arrays by design, so the prior
/// "var_val == 0 → skip" rule silently dropped every array name
/// from the top-level scan (Codex P2 review).  Follows ``VAR_LINK``
/// to the terminal record so a global aliased to another global is
/// reported when its target exists.
fn root_var_exists(var_addr: u32) bool {
    if (var_addr == 0) return false;
    const v: *const tcl_ns.Var = @ptrFromInt(var_addr);
    if ((v.flags & tcl_ns.VAR_LINK) != 0) {
        // ``var_get_scalar`` walks the link chain; for the terminal
        // we still need to distinguish array-vs-scalar, so resolve
        // the chain ourselves and re-probe.
        const terminal = tcl_ns.var_resolve_link(var_addr);
        if (terminal == 0) return false;
        return root_var_exists(terminal);
    }
    if ((v.flags & tcl_ns.VAR_ARRAY) != 0) return v.value != 0;
    return tcl_ns.var_get_scalar(var_addr) != 0;
}

/// Top-level fallback for ``info vars`` when no proc frame is active —
/// merge scalar globals from the root namespace's ``var_table`` with
/// top-level (un-namespaced) array names from the array directory.
/// Mirrors the qualified branch above but without the ``::`` prefix,
/// since the top-level scope is the root namespace itself.  Arrays
/// live in :mod:`tcl_array`'s directory rather than the var_table, so
/// a var-table-only scan would silently drop ``array set arr {...}``
/// entries (Codex P2 review).
fn root_namespace_names_matching(pat_ptr: u32, pat_len: u32, has_pattern: bool) i32 {
    const tcl_array = @import("../valtypes/tcl_array.zig");

    // Pull the top-level array names through the directory iterator
    // — already glob-matched and ``::__local::*``-filtered.
    const arr_list = tcl_array.array_top_level_names_matching(pat_ptr, pat_len);
    const arr_s = obj_ensure_string(arr_list);

    const root = tcl_ns.ns_root();
    const ns: *const tcl_ns.Namespace = @ptrFromInt(root);

    // Size pass over the scalar half so we can right-size the output
    // buffer up front; the merge loop appends without further alloc.
    var scalar_total: u32 = 0;
    var scalar_count: u32 = 0;
    if (ns.var_table.buf != 0) {
        var i: u32 = 0;
        while (i < ns.var_table.cap) : (i += 1) {
            const bucket = ns.var_table.buf + i * tcl_ns.NS_BUCKET_SIZE;
            const name_ptr: u32 = @bitCast(read_i32(bucket));
            if (name_ptr == 0) continue;
            const name_len: u32 = @bitCast(read_i32(bucket + 4));
            const var_addr: u32 = @bitCast(read_i32(bucket + tcl_ns.OFF_HANDLE));
            if (!root_var_exists(var_addr)) continue;
            if (has_pattern and !tcl_string.glob_match(pat_ptr, pat_len, name_ptr, name_len)) continue;
            if (scalar_count > 0) scalar_total += 1;
            scalar_total += name_len;
            scalar_count += 1;
        }
    }

    if (arr_s.len == 0 and scalar_count == 0) return obj_new_string(0, 0);
    if (scalar_count == 0) return arr_list;

    const arr_sep: u32 = if (arr_s.len > 0) @as(u32, 1) else @as(u32, 0);
    const merge_total: u32 = arr_s.len + arr_sep + scalar_total;
    const buf = alloc(merge_total);
    if (buf == 0) return obj_new_string(0, 0);
    var off: u32 = 0;
    if (arr_s.len > 0) {
        memcpy(buf, arr_s.ptr, arr_s.len);
        off = arr_s.len;
    }
    var written: u32 = if (arr_s.len > 0) 1 else 0;
    if (ns.var_table.buf != 0) {
        var i: u32 = 0;
        while (i < ns.var_table.cap) : (i += 1) {
            const bucket = ns.var_table.buf + i * tcl_ns.NS_BUCKET_SIZE;
            const name_ptr: u32 = @bitCast(read_i32(bucket));
            if (name_ptr == 0) continue;
            const name_len: u32 = @bitCast(read_i32(bucket + 4));
            const var_addr: u32 = @bitCast(read_i32(bucket + tcl_ns.OFF_HANDLE));
            if (!root_var_exists(var_addr)) continue;
            if (has_pattern and !tcl_string.glob_match(pat_ptr, pat_len, name_ptr, name_len)) continue;
            if (written > 0) {
                const dp: [*]u8 = @ptrFromInt(buf + off);
                dp[0] = ' ';
                off += 1;
            }
            memcpy(buf + off, name_ptr, name_len);
            off += name_len;
            written += 1;
        }
    }
    return obj.obj_new_string_take(buf, merge_total, merge_total);
}

// -- Dispatch --------------------------------------------------------------

/// Dispatch for 'info' command — one-word / two-word cases.  The
/// multi-arg cases (``info default``) are routed via a separate
/// entry; the interpreter's ``info`` handler picks whichever
/// dispatcher matches the arity.
pub export fn info_dispatch(subcmd: i32, arg: i32) i32 {
    const sub = obj_ensure_string(subcmd);
    if (sub.len == 0) return obj_new_string(0, 0);
    const sp: [*]const u8 = @ptrFromInt(sub.ptr);

    if (str_eq(sp, sub.len, "exists")) return info_exists(arg);
    if (str_eq(sp, sub.len, "body")) return info_body(arg);
    if (str_eq(sp, sub.len, "args")) return info_args(arg);
    if (str_eq(sp, sub.len, "complete")) return info_complete(arg);
    if (str_eq(sp, sub.len, "nameofexecutable")) return info_nameofexecutable();
    if (str_eq(sp, sub.len, "tclversion")) return info_tclversion();
    if (str_eq(sp, sub.len, "patchlevel")) return info_patchlevel();
    if (str_eq(sp, sub.len, "hostname")) return info_hostname();
    if (str_eq(sp, sub.len, "commands")) return info_commands(arg);
    if (str_eq(sp, sub.len, "procs")) return info_procs(arg);
    if (str_eq(sp, sub.len, "level")) return info_level(arg);
    if (str_eq(sp, sub.len, "script")) return info_script();
    if (str_eq(sp, sub.len, "vars")) return info_vars(arg);
    if (str_eq(sp, sub.len, "locals")) {
        // ``info locals`` returns only local variables of the current
        // proc frame (no globals).  Outside a proc, an empty string.
        // Matches Tcl 9 semantics for info-12.x.
        if (frames.frame_depth == 0) return obj_new_string(0, 0);
        var pat_ptr: u32 = 0;
        var pat_len: u32 = 0;
        var has_pattern = false;
        if (arg != 0) {
            const ps = obj_ensure_string(arg);
            pat_ptr = ps.ptr;
            pat_len = ps.len;
            has_pattern = true;
        }
        return frame_locals_names_matching(pat_ptr, pat_len, has_pattern);
    }
    if (str_eq(sp, sub.len, "frame")) return info_frame(arg);
    if (str_eq(sp, sub.len, "cmdcount")) {
        // Mirrors C Tcl ``Interp.cmdCount`` — incremented per
        // ``eval_command`` dispatch in ``tcl_interp.zig``.  Compiled
        // procs that take the AOT fast path don't increment;
        // inline arithmetic and direct local accesses also don't.
        // Matches the spec's "interpreter-visible commands" semantic
        // in practice for tcltest's info-3.x cases.
        const interp = @import("../interp/tcl_interp.zig");
        return obj_new_int(interp.cmd_count);
    }
    if (str_eq(sp, sub.len, "coroutine")) {
        // ``info coroutine`` returns the FQN of the currently
        // executing coroutine, or the empty string outside a
        // coroutine context.  coroutine.test 11.1 feeds this
        // into ``::tcl::unsupported::corotype``; previously the
        // missing branch returned empty and corotype raised
        // ``can only get coroutine type of a coroutine``.
        const coro_mod = @import("../sched/tcl_coro.zig");
        if (coro_mod.current_coro_name()) |span| {
            return obj.obj_new_string_copy(span.ptr, span.len);
        }
        return obj_new_string(0, 0);
    }
    return obj_new_string(0, 0);
}
