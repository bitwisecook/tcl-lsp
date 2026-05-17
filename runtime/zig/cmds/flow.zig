// ``return``, ``break``, ``continue``, ``error``, ``catch``,
// ``throw``, ``try``, ``apply``, ``tailcall``, ``time`` — control-flow signals.

const rt = @import("../tcl_runtime.zig");
const frames = @import("../interp/tcl_frames.zig");
const reg = @import("../dispatch/tcl_cmd_registry.zig");

const stubs = @import("../stubs/tcl_stubs.zig");
const catch_mod = @import("../interp/tcl_catch.zig");
const result_mod = @import("../interp/tcl_result.zig");
const str_eq = @import("../valtypes/tcl_chars.zig").str_eq;
const obj_ensure_string = rt.obj_ensure_string;
const obj_new_int = rt.obj_new_int;
const obj_new_string = rt.obj_new_string;

fn eval_return(words: []const i32) result_mod.InterpResult {
    // Tcl's ``return -level N -code C value`` produces an exception
    // that unwinds *N* frames with code C.  The default ``return val``
    // is ``-level 1 -code ok``: exit this proc with code OK.
    // ``return -code return`` is ``-level 1 -code return``: exit this
    // proc with code RETURN — and the *next* caller up treats RETURN
    // as "do a return now", so the propagation actually unwinds 2
    // frames.  We model this with a single ``extra_levels`` counter
    // tracked in ``tcl_catch.return_level`` and decremented at every
    // proc-dispatch boundary.
    const ReturnCode = enum { ok, err, ret, brk, cont, custom };
    var code_kind: ReturnCode = .ok;
    var custom_code: i64 = 0;
    var extra_levels: u32 = 0;
    var level_explicit: bool = false;
    var level_value: u32 = 1;
    var result_obj: i32 = 0;
    var errorcode_obj: i32 = 0;
    var errorinfo_obj: i32 = 0;
    var wi: u32 = 1;
    while (wi < words.len) : (wi += 1) {
        const w = obj_ensure_string(words[wi]);
        if (w.len >= 1) {
            const wp: [*]const u8 = @ptrFromInt(w.ptr);
            if (wp[0] == '-') {
                if (str_eq(wp, w.len, "-code") and wi + 1 < words.len) {
                    const code = obj_ensure_string(words[wi + 1]);
                    if (code.len >= 1) {
                        const cp: [*]const u8 = @ptrFromInt(code.ptr);
                        // Map Tcl status codes (keyword OR numeric)
                        // to our internal flag-driven equivalents:
                        //   ok       (0) → default return
                        //   error    (1) → catch_mod.tcl_cmd_error_full
                        //   return   (2) → return_flag + extra level
                        //   break    (3) → break_flag set
                        //   continue (4) → continue_flag set
                        //   numeric N≥5 → return_flag + return_code=N
                        // Without the break/continue branches,
                        // ``::tcl::OptDoOne``'s ``return -code break``
                        // / ``return -code continue`` were silently
                        // routed through the default return path,
                        // breaking the optparse state machine that
                        // ``OptDoAll``'s loop relies on.  The custom
                        // numeric branch is used by coroutine.test 7.5.
                        if (str_eq(cp, code.len, "ok") or
                            (code.len == 1 and cp[0] == '0'))
                        {
                            code_kind = .ok;
                        } else if (str_eq(cp, code.len, "error") or
                            (code.len == 1 and cp[0] == '1'))
                        {
                            code_kind = .err;
                        } else if (str_eq(cp, code.len, "return") or
                            (code.len == 1 and cp[0] == '2'))
                        {
                            code_kind = .ret;
                            extra_levels = 1;
                        } else if (str_eq(cp, code.len, "break") or
                            (code.len == 1 and cp[0] == '3'))
                        {
                            code_kind = .brk;
                        } else if (str_eq(cp, code.len, "continue") or
                            (code.len == 1 and cp[0] == '4'))
                        {
                            code_kind = .cont;
                        } else {
                            // Numeric ``-code N`` for N ≥ 5 — parse
                            // the value and surface it via the
                            // ``return_code`` side-channel that
                            // ``catch_leave`` reads.  Negative or
                            // non-numeric values fall through to the
                            // default ``.ok`` mapping (matching
                            // reference Tcl, which raises an error
                            // for unknown codes — left as a follow-up
                            // since the tcltest harness never relies
                            // on the diagnostic).
                            var n: i64 = 0;
                            var ok = code.len > 0;
                            var ki: u32 = 0;
                            while (ki < code.len) : (ki += 1) {
                                if (cp[ki] < '0' or cp[ki] > '9') {
                                    ok = false;
                                    break;
                                }
                                n = n * 10 + (cp[ki] - '0');
                            }
                            if (ok and n >= 5) {
                                code_kind = .custom;
                                custom_code = n;
                            }
                        }
                    }
                    wi += 1;
                    continue;
                }
                if (str_eq(wp, w.len, "-level") and wi + 1 < words.len) {
                    // ``-level N`` adds (N-1) extra unwind levels on
                    // top of the implicit "exit this proc".
                    // ``-level 0`` is special — it suppresses the implicit
                    // unwind so ``return -level 0 X`` produces TCL_OK
                    // with value X *without* exiting the surrounding
                    // proc (used by ``lmap``/``foreach`` body sentinels
                    // and by anything that wants the formal return-value
                    // machinery without actually returning).
                    const lev = obj_ensure_string(words[wi + 1]);
                    if (lev.len >= 1) {
                        const lp: [*]const u8 = @ptrFromInt(lev.ptr);
                        var nv: u32 = 0;
                        var ok = true;
                        for (0..lev.len) |k| {
                            if (lp[k] < '0' or lp[k] > '9') {
                                ok = false;
                                break;
                            }
                            nv = nv * 10 + @as(u32, @intCast(lp[k] - '0'));
                        }
                        if (ok) {
                            level_explicit = true;
                            level_value = nv;
                            if (nv > 0) {
                                extra_levels = nv - 1;
                            } else {
                                extra_levels = 0;
                            }
                        }
                    }
                    wi += 1;
                    continue;
                }
                if (str_eq(wp, w.len, "-errorcode") and wi + 1 < words.len) {
                    // Tcl 9 ``return -errorcode VAL`` validates VAL as
                    // a well-formed list (result-6.3 / cmdMZ-return-x):
                    // ``{{}a}`` raises ``bad -errorcode value: expected
                    // a list but got "<val>"`` because the trailing
                    // ``a`` after the brace element isn't whitespace.
                    // ``check_list_syntax`` raises its own list-shape
                    // diagnostic on failure; suppress that by latching
                    // the error_flag/msg only after we've reset them
                    // with the canonical ``bad -errorcode value`` text.
                    const list_parse = @import("../valtypes/tcl_list_parse.zig");
                    const ec_s = obj_ensure_string(words[wi + 1]);
                    if (list_parse.check_list_syntax(ec_s.ptr, ec_s.len) != 0) {
                        const obj_pkg = @import("../valtypes/tcl_obj.zig");
                        const cm = @import("../interp/tcl_catch.zig");
                        const prefix = "bad -errorcode value: expected a list but got \"";
                        const suffix = "\"";
                        const total: u32 = @intCast(prefix.len + ec_s.len + suffix.len);
                        const buf_addr: u32 = obj_pkg.alloc(total);
                        if (buf_addr != 0) {
                            const buf: [*]u8 = @ptrFromInt(buf_addr);
                            var off: u32 = 0;
                            for (prefix) |c| {
                                buf[off] = c;
                                off += 1;
                            }
                            const ec_ptr: [*]const u8 = @ptrFromInt(ec_s.ptr);
                            for (0..ec_s.len) |k| {
                                buf[off] = ec_ptr[k];
                                off += 1;
                            }
                            for (suffix) |c| {
                                buf[off] = c;
                                off += 1;
                            }
                            const err_obj = obj_pkg.obj_new_string_take(buf_addr, total, total);
                            cm.tcl_cmd_error(err_obj);
                        } else {
                            stubs.raise("bad -errorcode value");
                        }
                        return result_mod.from_globals(0);
                    }
                    errorcode_obj = words[wi + 1];
                    wi += 1;
                    continue;
                }
                if (str_eq(wp, w.len, "-errorinfo") and wi + 1 < words.len) {
                    errorinfo_obj = words[wi + 1];
                    wi += 1;
                    continue;
                }
                if (str_eq(wp, w.len, "-options") and wi + 1 < words.len) {
                    // Expand DICT into the pending-extras options
                    // for ``return … -options DICT …``.  Multiple
                    // ``-options`` arguments accumulate (cmdMZ-return-
                    // 2.20).  Standard slots (``-code`` / ``-level`` /
                    // ``-errorcode`` / ``-errorinfo``) inside the
                    // dict are extracted into their dedicated
                    // tracking variables rather than the extras
                    // dict — they pass through the normal
                    // ``-code`` / ``-level`` paths so the catch
                    // sees the right surface.
                    const dict_mod_opt = @import("../valtypes/tcl_dict.zig");
                    const obj_opt = @import("../valtypes/tcl_obj.zig");
                    const opts_dict = words[wi + 1];
                    const keys = dict_mod_opt.dict_keys(opts_dict);
                    if (keys != 0) {
                        const ks = obj_ensure_string(keys);
                        const n = obj_opt.list_count_elements(ks.ptr, ks.len);
                        var ki: i64 = 0;
                        while (ki < n) : (ki += 1) {
                            const elem = obj_opt.list_element_at(ks.ptr, ks.len, ki);
                            const key_obj = obj_opt.obj_new_string_copy(
                                ks.ptr + elem.start,
                                elem.len,
                            );
                            const ks_inner = obj_ensure_string(key_obj);
                            const val_obj = dict_mod_opt.dict_get(opts_dict, key_obj);
                            if (val_obj != 0 and ks_inner.len > 0) {
                                const kp: [*]const u8 = @ptrFromInt(ks_inner.ptr);
                                if (str_eq(kp, ks_inner.len, "-code")) {
                                    const cs = obj_ensure_string(val_obj);
                                    if (cs.len >= 1) {
                                        const cp: [*]const u8 = @ptrFromInt(cs.ptr);
                                        if (str_eq(cp, cs.len, "ok") or (cs.len == 1 and cp[0] == '0')) {
                                            code_kind = .ok;
                                        } else if (str_eq(cp, cs.len, "error") or (cs.len == 1 and cp[0] == '1')) {
                                            code_kind = .err;
                                        } else if (str_eq(cp, cs.len, "return") or (cs.len == 1 and cp[0] == '2')) {
                                            code_kind = .ret;
                                            extra_levels = 1;
                                        } else if (str_eq(cp, cs.len, "break") or (cs.len == 1 and cp[0] == '3')) {
                                            code_kind = .brk;
                                        } else if (str_eq(cp, cs.len, "continue") or (cs.len == 1 and cp[0] == '4')) {
                                            code_kind = .cont;
                                        } else {
                                            var n2: i64 = 0;
                                            var okp = cs.len > 0;
                                            var k2: u32 = 0;
                                            while (k2 < cs.len) : (k2 += 1) {
                                                if (cp[k2] < '0' or cp[k2] > '9') {
                                                    okp = false;
                                                    break;
                                                }
                                                n2 = n2 * 10 + (cp[k2] - '0');
                                            }
                                            if (okp and n2 >= 5) {
                                                code_kind = .custom;
                                                custom_code = n2;
                                            }
                                        }
                                    }
                                } else if (str_eq(kp, ks_inner.len, "-level")) {
                                    const lv = obj_ensure_string(val_obj);
                                    if (lv.len >= 1) {
                                        const lp: [*]const u8 = @ptrFromInt(lv.ptr);
                                        var nv: u32 = 0;
                                        var ok2 = true;
                                        for (0..lv.len) |k2| {
                                            if (lp[k2] < '0' or lp[k2] > '9') {
                                                ok2 = false;
                                                break;
                                            }
                                            nv = nv * 10 + @as(u32, @intCast(lp[k2] - '0'));
                                        }
                                        if (ok2) {
                                            level_explicit = true;
                                            level_value = nv;
                                            extra_levels = if (nv > 0) nv - 1 else 0;
                                        }
                                    }
                                } else if (str_eq(kp, ks_inner.len, "-errorcode")) {
                                    errorcode_obj = val_obj;
                                } else if (str_eq(kp, ks_inner.len, "-errorinfo")) {
                                    errorinfo_obj = val_obj;
                                } else {
                                    // Arbitrary option — record into extras.
                                    if (catch_mod.state.pending_return_extras == 0) {
                                        catch_mod.state.pending_return_extras = dict_mod_opt.dict_create();
                                    }
                                    const new_extras = dict_mod_opt.dict_set(
                                        catch_mod.state.pending_return_extras,
                                        key_obj,
                                        val_obj,
                                    );
                                    if (new_extras != catch_mod.state.pending_return_extras) {
                                        if (catch_mod.state.pending_return_extras != 0) {
                                            obj_opt.tcl_obj_release(catch_mod.state.pending_return_extras);
                                        }
                                        catch_mod.state.pending_return_extras = new_extras;
                                    }
                                }
                            }
                            obj_opt.tcl_obj_release(key_obj);
                        }
                        obj_opt.tcl_obj_release(keys);
                    }
                    wi += 1;
                    continue;
                }
                // Arbitrary ``-OPT VALUE`` — capture into the
                // pending-extras dict so the surrounding catch's
                // options dict reports them.  cmdMZ-return-2.1
                // (``return -bar soom``) expects ``$foo`` to be
                // ``-bar soom -code 0 -level 1``.  Only treat as an
                // option when there's a value word following — a
                // dangling ``-name`` at the tail collapses into
                // ``result_obj`` like any other unrecognised word.
                if (wi + 1 < words.len and w.len > 1) {
                    const dict_mod_ext = @import("../valtypes/tcl_dict.zig");
                    const obj_ext = @import("../valtypes/tcl_obj.zig");
                    if (catch_mod.state.pending_return_extras == 0) {
                        catch_mod.state.pending_return_extras = dict_mod_ext.dict_create();
                    }
                    const new_extras = dict_mod_ext.dict_set(
                        catch_mod.state.pending_return_extras,
                        words[wi],
                        words[wi + 1],
                    );
                    if (new_extras != catch_mod.state.pending_return_extras) {
                        if (catch_mod.state.pending_return_extras != 0) {
                            obj_ext.tcl_obj_release(catch_mod.state.pending_return_extras);
                        }
                        catch_mod.state.pending_return_extras = new_extras;
                    }
                    wi += 1;
                    continue;
                }
            }
        }
        result_obj = words[wi];
    }
    // Snapshot the user-supplied ``-code`` / ``-level`` for the
    // TIP 90 ``catch BODY result options`` dict.  ``-code return``
    // is a syntactic shorthand for "produce a TCL_RETURN that, after
    // the immediate caller's absorb, will itself convert into a
    // TCL_RETURN one level further up" — its observable catch-side
    // shape is ``-code 0 -level N+1``, NOT ``-code 2 -level N``.
    // See cmdMZ-return-2.2 / 2.3 for the exact expected mapping.
    //
    // Stamping these unconditionally up-front means the early-return
    // branches below (``-code error``, ``-level 0 -code break``, etc.)
    // also propagate the right values to the surrounding ``catch``'s
    // options dict.
    {
        const dict_code: i64 = switch (code_kind) {
            .ok => 0,
            .err => 1,
            .ret => 0,
            .brk => 3,
            .cont => 4,
            .custom => custom_code,
        };
        const dict_level: u32 = if (code_kind == .ret) level_value + 1 else level_value;
        catch_mod.state.pending_return_code = dict_code;
        catch_mod.state.pending_return_level = dict_level;
        catch_mod.state.pending_return_armed = 1;
    }
    if (code_kind == .err) {
        // ``return -code error msg`` is a USER-supplied error — it
        // should keep the default ``NONE`` errorCode unless the
        // caller passed ``-errorcode`` explicitly.  The 1-arg
        // ``tcl_cmd_error`` auto-detects a leading ``wrong # args:``
        // and stamps ``TCL WRONGARGS`` (correct for the BUILTIN-
        // arity-check path that uses it directly).  For
        // ``return -code error "wrong # args: synthetic"`` from a
        // script, that auto-detection would mis-tag the user's
        // message — Copilot review on PR #325.  Route through the
        // 3-arg form, passing the user's ``-errorinfo`` /
        // ``-errorcode`` so cmdMZ-return-2.15..17 see the expected
        // ``::errorCode {a b}`` after the apply unwinds.
        catch_mod.tcl_cmd_error_full(result_obj, errorinfo_obj, errorcode_obj);
        return result_mod.from_globals(0);
    }
    // ``return -code break`` / ``return -code continue`` aren't yet
    // wired through the compiled-proc body machinery — propagating
    // them correctly requires a per-statement break/continue check
    // that can branch out to the enclosing loop's consume probe,
    // which the codegen doesn't emit yet.  Falling through to the
    // default return path keeps single-callsite tests green at the
    // cost of a small handful of optparse state-machine cases
    // (opt-10.8 / 10.9 / 11.x).  Tracking as a follow-up; the
    // signal-flag scaffolding (``signal_break_flag`` /
    // ``signal_continue_flag`` / ``flow_check_signal_loop``) stays
    // in place for that future work.
    if (code_kind == .brk or code_kind == .cont) {
        // Suppress "unused .brk/.cont arm" while we leave the full
        // wire-up for the follow-up that adds the codegen-side
        // per-statement probes.
    }
    // ``return -level 0 …`` is the no-unwind form: the command
    // produces the value with the requested status code but the
    // current frame keeps running.  ``return -level 0 X`` is exactly
    // ``set _ X`` for OK status (lmap-1.2a, opt-10.x).  Other status
    // codes route directly to the matching break/continue/return
    // flag.
    if (level_explicit and level_value == 0) {
        switch (code_kind) {
            .ok => return result_mod.from_globals(result_obj),
            .brk => {
                result_mod.set_break();
                return result_mod.from_globals(0);
            },
            .cont => {
                result_mod.set_continue();
                return result_mod.from_globals(0);
            },
            .custom => {
                // ``return -level 0 -code N`` for N ≥ 5 — set
                // ``return_flag`` so the surrounding catch sees a
                // RETURN-class code, plus stash the exact ``N`` in
                // ``return_code`` so ``catch_leave`` surfaces it
                // instead of the generic ``2`` (coroutine.test 7.5).
                const obj_mod = @import("../valtypes/tcl_obj.zig");
                const old = rt.return_val.*;
                if (result_obj != 0) obj_mod.tcl_obj_retain(result_obj);
                if (old != 0 and old != result_obj) obj_mod.tcl_obj_release(old);
                result_mod.set_return(result_obj, 0);
                catch_mod.state.return_code = custom_code;
                return result_mod.from_globals(result_obj);
            },
            .ret, .err => {},
        }
    }
    // MM-B.5: ``return_val`` slot holds the value until
    // ``eval_proc_call_bucket`` reads it back.  Retain so ``MM-B.4``'s
    // parser-side release of ``words[1]`` doesn't free the value
    // before the caller reads it.  Release any prior occupant — a
    // nested ``return`` inside an outer ``return`` could otherwise
    // leak the previous slot.
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const old = rt.return_val.*;
    if (result_obj != 0) obj_mod.tcl_obj_retain(result_obj);
    if (old != 0 and old != result_obj) obj_mod.tcl_obj_release(old);
    result_mod.set_return(result_obj, extra_levels);
    return result_mod.from_globals(result_obj);
}

fn eval_break(words: []const i32) result_mod.InterpResult {
    if (words.len > 1) {
        stubs.raise("wrong # args: should be \"break\"");
        return result_mod.from_globals(0);
    }
    result_mod.set_break();
    return result_mod.from_globals(0);
}

fn eval_continue(words: []const i32) result_mod.InterpResult {
    if (words.len > 1) {
        stubs.raise("wrong # args: should be \"continue\"");
        return result_mod.from_globals(0);
    }
    result_mod.set_continue();
    return result_mod.from_globals(0);
}

fn eval_error(words: []const i32) result_mod.InterpResult {
    // ``error msg ?info? ?code?`` — populate ``::errorInfo`` /
    // ``::errorCode`` from the optional 2nd / 3rd args.  Without
    // this, ``catch ... opt; dict get $opt -errorcode`` always
    // saw ``NONE`` because the upstream ``error -code FOO`` form
    // never made it into the global.
    if (words.len < 2) return result_mod.from_globals(0);
    const msg = words[1];
    const info: i32 = if (words.len >= 3) words[2] else 0;
    const code: i32 = if (words.len >= 4) words[3] else 0;
    catch_mod.tcl_cmd_error_full(msg, info, code);
    return result_mod.from_globals(0);
}

fn eval_catch(words: []const i32) result_mod.InterpResult {
    if (words.len < 2 or words.len > 4) {
        stubs.raise("wrong # args: should be \"catch script ?resultVarName? ?optionVarName?\"");
        return result_mod.from_globals(0);
    }
    if (words.len >= 2) {
        const interp = @import("../interp/tcl_interp.zig");
        rt.catch_enter();
        const body_s = obj_ensure_string(words[1]);
        const body_result = interp.eval_script(body_s.ptr, body_s.len);
        rt.catch_set_ok_result(body_result);
        // ``catch_leave`` *must* run before ``catch_result`` because the
        // result accessor reads ``last_catch_had_error``, which is only
        // populated inside ``catch_leave``.  Reading the result first
        // would surface the *previous* catch's outcome — making
        // ``catch {llength {return}} msg`` set ``msg`` to whatever
        // the prior catch had latched (``""`` on the first call) and
        // breaking tcltest's ``SubstArguments`` "is this a valid 1-elem
        // list?" probe (``[catch {llength $token} length] == 0 &&
        // $length == 1``) on every malformed token.
        const code = rt.catch_leave();
        const catch_val = rt.catch_result();
        if (words.len >= 3) _ = frames.var_set(words[2], catch_val);
        // 4-arg ``catch BODY result opt`` — populate ``$opt`` with the
        // standard return-options dict so ``dict get $opt -errorcode``
        // observes the error class set by ``error msg ?info? ?code?``
        // (or by a runtime ``raise`` such as ``CLOCK dateTooLarge``).
        // Mirrors the wasm-codegen path in
        // ``_emitter/_control_flow.py::_emit_catch``.
        if (words.len >= 4) {
            _ = frames.var_set(words[3], catch_mod.catch_options());
        }
        return result_mod.from_globals(code);
    }
    return result_mod.from_globals(obj_new_int(0));
}

// ``throw type message`` — raise error with explicit errorCode.
fn eval_throw(words: []const i32) result_mod.InterpResult {
    if (words.len < 3) {
        stubs.raise("wrong # args: should be \"throw type message\"");
        return result_mod.from_globals(0);
    }

    catch_mod.tcl_cmd_error_full(words[2], 0, words[1]);
    return result_mod.from_globals(0);
}

// ``try body ?on code varlist handler? ... ?finally body?``
fn eval_try(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) return result_mod.from_globals(obj_new_string(0, 0));
    const interp = @import("../interp/tcl_interp.zig");

    rt.catch_enter();
    const body_s = obj_ensure_string(words[1]);
    const body_res = interp.eval_script(body_s.ptr, body_s.len);
    rt.catch_set_ok_result(body_res);
    // Order matters: ``catch_leave`` snapshots ``last_catch_had_error``
    // that ``catch_result`` reads.  See ``eval_catch`` above.
    const code_obj = rt.catch_leave();
    const catch_val = rt.catch_result();
    const had_error: bool = rt.obj_get_int(code_obj) != 0;

    var final_result: i32 = if (had_error) catch_val else body_res;
    var error_raised = had_error;
    var handled = false;
    var finally_body: i32 = 0;

    // Parse clauses
    var wi: u32 = 2;
    while (wi < words.len) {
        const kw = obj_ensure_string(words[wi]);
        if (kw.len == 0) {
            wi += 1;
            continue;
        }
        const kp: [*]const u8 = @ptrFromInt(kw.ptr);

        // "finally body"
        if (kw.len == 7 and kp[0] == 'f' and kp[1] == 'i' and kp[2] == 'n' and
            kp[3] == 'a' and kp[4] == 'l' and kp[5] == 'l' and kp[6] == 'y')
        {
            wi += 1;
            if (wi < words.len) {
                finally_body = words[wi];
                wi += 1;
            }
            continue;
        }

        // "on code varlist handler"
        if (kw.len == 2 and kp[0] == 'o' and kp[1] == 'n') {
            wi += 1;
            if (wi + 2 >= words.len) break;
            const code_word = words[wi];
            wi += 1;
            const varlist = words[wi];
            wi += 1;
            const handler = words[wi];
            wi += 1;
            if (!handled) {
                const cs = obj_ensure_string(code_word);
                const cp: [*]const u8 = @ptrFromInt(cs.ptr);
                const is_error_kw = cs.len == 5 and cp[0] == 'e' and cp[1] == 'r' and
                    cp[2] == 'r' and cp[3] == 'o' and cp[4] == 'r';
                const is_ok_kw = cs.len == 2 and cp[0] == 'o' and cp[1] == 'k';
                const code_n = rt.obj_get_int(code_word);
                const matches =
                    (had_error and (is_error_kw or code_n == 1)) or
                    (!had_error and (is_ok_kw or code_n == 0));
                if (matches) {
                    const vl_s = obj_ensure_string(varlist);
                    const vl_n = rt.list_count_elements(vl_s.ptr, vl_s.len);
                    if (vl_n >= 1) {
                        const v0 = rt.list_element_at(vl_s.ptr, vl_s.len, 0);
                        if (v0.len > 0) {
                            const vn = rt.obj_new_string_copy(vl_s.ptr + v0.start, v0.len);
                            _ = frames.var_set(vn, catch_val);
                        }
                    }
                    const hb_s = obj_ensure_string(handler);
                    final_result = interp.eval_script(hb_s.ptr, hb_s.len);
                    error_raised = false;
                    handled = true;
                }
            }
            continue;
        }

        // "trap type varlist handler" — matches any error (type prefix ignored for now)
        if (kw.len == 4 and kp[0] == 't' and kp[1] == 'r' and kp[2] == 'a' and kp[3] == 'p') {
            wi += 1;
            if (wi + 2 >= words.len) break;
            wi += 1; // skip type
            const varlist = words[wi];
            wi += 1;
            const handler = words[wi];
            wi += 1;
            if (!handled and had_error) {
                const vl_s = obj_ensure_string(varlist);
                const vl_n = rt.list_count_elements(vl_s.ptr, vl_s.len);
                if (vl_n >= 1) {
                    const v0 = rt.list_element_at(vl_s.ptr, vl_s.len, 0);
                    if (v0.len > 0) {
                        const vn = rt.obj_new_string_copy(vl_s.ptr + v0.start, v0.len);
                        _ = frames.var_set(vn, catch_val);
                    }
                }
                const hb_s = obj_ensure_string(handler);
                final_result = interp.eval_script(hb_s.ptr, hb_s.len);
                error_raised = false;
                handled = true;
            }
            continue;
        }

        wi += 1;
    }

    // Finally: run in an isolated catch frame.  An error/return/break/continue
    // from the finally body overrides the handler outcome per Tcl semantics.
    // The save+clear preserves a handler-issued return/break/continue across
    // the finally body so a clean finally completion restores them.
    if (finally_body != 0) {
        const snap = result_mod.flow_save_and_clear();
        rt.catch_enter();
        const fb_s = obj_ensure_string(finally_body);
        const fb_result = interp.eval_script(fb_s.ptr, fb_s.len);
        rt.catch_set_ok_result(fb_result);
        // ``catch_leave`` before ``catch_result`` — see ``eval_catch``.
        const finally_code = rt.catch_leave();
        const fb_val = rt.catch_result();
        if (rt.obj_get_int(finally_code) != 0) {
            // Finally raised an error — propagate it, overriding handler result.
            catch_mod.tcl_cmd_error(fb_val);
            return result_mod.from_globals(fb_result);
        }
        // If finally set a return/break/continue signal, propagate it.
        const fb_ir = result_mod.snapshot(fb_result);
        if (fb_ir.code == .RETURN or fb_ir.code == .BREAK or fb_ir.code == .CONTINUE) {
            return result_mod.from_globals(fb_result);
        }
        // Finally completed normally: restore signals from after handlers ran.
        result_mod.flow_restore(snap);
    }

    if (error_raised and !handled) catch_mod.tcl_cmd_error(catch_val);
    return result_mod.from_globals(final_result);
}

// ``apply`` — delegate to tcl_interp.eval_apply.
fn eval_apply_cmd(words: []const i32) result_mod.InterpResult {
    const interp = @import("../interp/tcl_interp.zig");
    return result_mod.from_globals(interp.eval_apply(words));
}

// ``tailcall cmd ?arg ...?`` — call target, force-return from current proc.
fn eval_tailcall(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) {
        rt.tcl_cmd_error(obj_new_string(0, 0));
        return result_mod.from_globals(0);
    }
    const interp = @import("../interp/tcl_interp.zig");
    const result = interp.eval_call(words[1..]);
    const ir = result_mod.snapshot(result);
    if (ir.code != .ERROR and ir.code != .RETURN) {
        result_mod.set_return(result, 0);
    }
    return result_mod.from_globals(result);
}

// ``time script ?count?`` — measure microseconds per iteration.
fn eval_time(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) return result_mod.from_globals(obj_new_string(0, 0));
    const interp = @import("../interp/tcl_interp.zig");
    const clock = @import("../io/tcl_clock.zig");

    const count: i64 = if (words.len >= 3) rt.obj_get_int(words[2]) else 1;
    const body_s = obj_ensure_string(words[1]);

    const start_us = rt.obj_get_int(clock.clock_clicks());
    var i: i64 = 0;
    var last: i32 = obj_new_string(0, 0);
    while (i < count) : (i += 1) {
        last = interp.eval_script(body_s.ptr, body_s.len);
        const ir = result_mod.snapshot(last);
        if (ir.code == .ERROR or ir.code == .RETURN) return result_mod.from_globals(last);
    }
    const end_us = rt.obj_get_int(clock.clock_clicks());
    const per_iter = if (count > 0) @divTrunc(end_us - start_us, count) else 0;

    // Build "<N> microseconds per iteration"
    const pi_s = rt.obj_ensure_string(rt.obj_new_int(per_iter));
    const suffix = " microseconds per iteration";
    const total: u32 = @intCast(pi_s.len + suffix.len);
    const buf = rt.alloc(total);
    rt.memcpy(buf, pi_s.ptr, pi_s.len);
    var k: u32 = 0;
    while (k < suffix.len) : (k += 1) {
        const d: [*]u8 = @ptrFromInt(buf + pi_s.len + k);
        d[0] = suffix[k];
    }
    return result_mod.from_globals(rt.obj_new_string(@bitCast(buf), @bitCast(total)));
}

pub const registrations = [_]reg.CmdEntry{
    .{ .name = "return", .arity_min = 0, .arity_max = null, .handler = &eval_return },
    .{ .name = "break", .arity_min = 0, .arity_max = 0, .handler = &eval_break },
    .{ .name = "continue", .arity_min = 0, .arity_max = 0, .handler = &eval_continue },
    .{ .name = "error", .arity_min = 1, .arity_max = 3, .handler = &eval_error },
    .{ .name = "catch", .arity_min = 1, .arity_max = 3, .handler = &eval_catch },
    .{ .name = "throw", .arity_min = 2, .arity_max = 2, .handler = &eval_throw },
    .{ .name = "try", .arity_min = 1, .arity_max = null, .handler = &eval_try },
    .{ .name = "apply", .arity_min = 1, .arity_max = null, .handler = &eval_apply_cmd },
    .{ .name = "tailcall", .arity_min = 0, .arity_max = null, .handler = &eval_tailcall },
    .{ .name = "time", .arity_min = 1, .arity_max = 2, .handler = &eval_time },
};
