// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

// ``return``, ``break``, ``continue``, ``error``, ``catch``,
// ``throw``, ``try``, ``apply``, ``tailcall``, ``time`` — control-flow signals.

const std = @import("std");
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

/// Read ``clock_clicks`` in microseconds, releasing the transient TclObj
/// it allocates.  ``clock_clicks`` returns a fresh ``obj_new_int``; the
/// callers only want the integer, so without this each timing sample
/// (``timerate``'s start / per-iteration / end reads) leaks one object.
fn clicks_us() i64 {
    const clk = @import("../io/tcl_clock.zig");
    const obj_lifecycle = @import("../valtypes/tcl_obj.zig");
    const o = clk.clock_clicks();
    const v = rt.obj_get_int(o);
    if (o != 0) obj_lifecycle.tcl_obj_release(o);
    return v;
}

/// Tcl status code for ``return -code`` / a ``-options`` dict's
/// ``-code`` key — keyword (``ok``/``error``/…) or numeric.
const ReturnCode = enum { ok, err, ret, brk, cont, custom };

/// Mutable state threaded through ``return``'s option parsing so the
/// recursive ``-options`` expander updates the same slots the main
/// argument loop uses.
const RetOptCtx = struct {
    code_kind: *ReturnCode,
    custom_code: *i64,
    extra_levels: *u32,
    level_explicit: *bool,
    level_value: *u32,
    errorcode_obj: *i32,
    errorinfo_obj: *i32,
    // Whether ``errorcode_obj`` / ``errorinfo_obj`` currently hold a
    // FRESH (+1 owned) object minted by the ``-options`` expander, as
    // opposed to a borrowed ``words[]`` slice set by the direct
    // ``-errorcode`` / ``-errorinfo`` argument path.  Owned slots are
    // released by ``eval_return`` after consumption (``tcl_cmd_error_full``
    // / ``global_set`` keep their own retain); borrowed ones must not be.
    errorcode_owned: *bool,
    errorinfo_owned: *bool,
};

/// Copy list element [*start*, *start*+*len*) out of *base_ptr* into a
/// fresh owning TclObj.  ``list_element_at`` already hands back the
/// brace-stripped content span for braced elements, so this matches
/// what ``dict_get``'s list-walk fallback produces.
fn ret_opt_elem_to_obj(base_ptr: u32, start: u32, len: u32) i32 {
    const obj_opt = @import("../valtypes/tcl_obj.zig");
    if (len == 0) return obj_opt.obj_new_string(0, 0);
    return obj_opt.obj_new_string_copy(base_ptr + start, len);
}

/// Apply one ``KEY VALUE`` pair from a ``-options`` dict.  The standard
/// slots (``-code`` / ``-level`` / ``-errorcode`` / ``-errorinfo``)
/// update their dedicated tracking variables; a nested ``-options``
/// recurses (C Tcl's ``TclMergeReturnOptions`` flattens recursively —
/// cmdMZ-return-2.21); anything else lands in the pending-extras dict.
///
/// Returns ``true`` when it *takes ownership* of *val_obj* (stores the
/// +1 handle into the ``errorcode_obj`` / ``errorinfo_obj`` slot for
/// consumption later by ``eval_return``); ``false`` when *val_obj* is
/// transient — read as a string (``-code`` / ``-level``), recursed into
/// (``-options``), or byte-copied by ``dict_set`` (extras) — in which
/// case the caller must release it.
fn apply_one_return_option(ctx: RetOptCtx, key_obj: i32, val_obj: i32, depth: u32) bool {
    const obj_opt = @import("../valtypes/tcl_obj.zig");
    const dict_mod_opt = @import("../valtypes/tcl_dict.zig");
    const ks_inner = obj_ensure_string(key_obj);
    if (val_obj == 0 or ks_inner.len == 0) return false;
    const kp: [*]const u8 = @ptrFromInt(ks_inner.ptr);
    if (str_eq(kp, ks_inner.len, "-options")) {
        apply_return_options_dict(ctx, val_obj, depth + 1);
        return false;
    }
    if (str_eq(kp, ks_inner.len, "-code")) {
        const cs = obj_ensure_string(val_obj);
        if (cs.len >= 1) {
            const cp: [*]const u8 = @ptrFromInt(cs.ptr);
            if (str_eq(cp, cs.len, "ok") or (cs.len == 1 and cp[0] == '0')) {
                ctx.code_kind.* = .ok;
            } else if (str_eq(cp, cs.len, "error") or (cs.len == 1 and cp[0] == '1')) {
                ctx.code_kind.* = .err;
            } else if (str_eq(cp, cs.len, "return") or (cs.len == 1 and cp[0] == '2')) {
                ctx.code_kind.* = .ret;
                ctx.extra_levels.* = 1;
            } else if (str_eq(cp, cs.len, "break") or (cs.len == 1 and cp[0] == '3')) {
                ctx.code_kind.* = .brk;
            } else if (str_eq(cp, cs.len, "continue") or (cs.len == 1 and cp[0] == '4')) {
                ctx.code_kind.* = .cont;
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
                    ctx.code_kind.* = .custom;
                    ctx.custom_code.* = n2;
                }
            }
        }
        return false;
    }
    if (str_eq(kp, ks_inner.len, "-level")) {
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
                ctx.level_explicit.* = true;
                ctx.level_value.* = nv;
                ctx.extra_levels.* = if (nv > 0) nv - 1 else 0;
            }
        }
        return false;
    }
    if (str_eq(kp, ks_inner.len, "-errorcode")) {
        // Take ownership of the +1 ``val_obj``.  Free a previously
        // expander-minted value before overwriting it
        // (``-options {-errorcode A -errorcode B}``); a borrowed
        // direct-path value (``errorcode_owned == false``) is left be.
        if (ctx.errorcode_obj.* != 0 and ctx.errorcode_owned.*) {
            obj_opt.tcl_obj_release(ctx.errorcode_obj.*);
        }
        ctx.errorcode_obj.* = val_obj;
        ctx.errorcode_owned.* = true;
        return true;
    }
    if (str_eq(kp, ks_inner.len, "-errorinfo")) {
        if (ctx.errorinfo_obj.* != 0 and ctx.errorinfo_owned.*) {
            obj_opt.tcl_obj_release(ctx.errorinfo_obj.*);
        }
        ctx.errorinfo_obj.* = val_obj;
        ctx.errorinfo_owned.* = true;
        return true;
    }
    // Arbitrary option — record into the pending-extras dict.  ``dict_set``
    // copies the key and value byte content (it never takes ownership of
    // the handles), so ``val_obj`` stays transient and the caller frees it.
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
    return false;
}

/// Expand a ``-options`` dict, applying each ``KEY VALUE`` pair in
/// order.  Iterates the raw list pairs (NOT ``dict_keys``, which dedups
/// — a dict with repeated keys like ``{-options A -options B}`` must
/// apply each occurrence; cmdMZ-return-2.21).  ``depth`` guards against
/// pathological self-referential nesting.  ``ret_opt_elem_to_obj`` mints
/// a fresh +1 ``key_obj`` / ``val_obj`` per pair: ``key_obj`` is always
/// released here, and ``val_obj`` is too — UNLESS
/// ``apply_one_return_option`` reports it took ownership (the
/// ``-errorcode`` / ``-errorinfo`` slots, freed later by ``eval_return``).
fn apply_return_options_dict(ctx: RetOptCtx, opts_dict: i32, depth: u32) void {
    if (depth > 64 or opts_dict == 0) return;
    const obj_opt = @import("../valtypes/tcl_obj.zig");
    const ds = obj_ensure_string(opts_dict);
    if (ds.len == 0) return;
    const n = obj_opt.list_count_elements(ds.ptr, ds.len);
    var i: i64 = 0;
    while (i + 1 < n) : (i += 2) {
        const ke = obj_opt.list_element_at(ds.ptr, ds.len, i);
        const ve = obj_opt.list_element_at(ds.ptr, ds.len, i + 1);
        const key_obj = ret_opt_elem_to_obj(ds.ptr, ke.start, ke.len);
        const val_obj = ret_opt_elem_to_obj(ds.ptr, ve.start, ve.len);
        const took_val = apply_one_return_option(ctx, key_obj, val_obj, depth);
        if (!took_val and val_obj != 0) obj_opt.tcl_obj_release(val_obj);
        if (key_obj != 0) obj_opt.tcl_obj_release(key_obj);
    }
}

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
    var code_kind: ReturnCode = .ok;
    var custom_code: i64 = 0;
    var extra_levels: u32 = 0;
    var level_explicit: bool = false;
    var level_value: u32 = 1;
    var result_obj: i32 = 0;
    var errorcode_obj: i32 = 0;
    var errorinfo_obj: i32 = 0;
    // ``errorcode_obj`` / ``errorinfo_obj`` hold either a BORROWED
    // ``words[]`` slice (direct ``-errorcode`` / ``-errorinfo`` args) or a
    // FRESH +1 object minted by the ``-options`` expander.  Track which so
    // the owned ones are freed (and the borrowed ones are not) via the
    // ``defer`` below: ``tcl_cmd_error_full`` / ``global_set`` keep their
    // own retain on consumption, and the non-error branches simply drop
    // the slots.
    var errorcode_owned: bool = false;
    var errorinfo_owned: bool = false;
    const obj_ret_cleanup = @import("../valtypes/tcl_obj.zig");
    defer {
        if (errorcode_owned and errorcode_obj != 0) obj_ret_cleanup.tcl_obj_release(errorcode_obj);
        if (errorinfo_owned and errorinfo_obj != 0) obj_ret_cleanup.tcl_obj_release(errorinfo_obj);
    }
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
                            // Numeric ``-code N`` outside the 0..4
                            // keyword range — parse the value and
                            // surface it via the ``return_code``
                            // side-channel that ``catch_leave`` reads.
                            // ``parse_code_number`` handles both decimal
                            // (positive AND negative, per interp-26.x)
                            // and hex ``0xNN`` forms (error-10.11 /
                            // -10.12 verify hex equivalence).  Tcl 9
                            // accepts every numeric value as a custom
                            // completion code; values that *also* land
                            // in the keyword range are pre-empted by
                            // the keyword branches above.  Non-numeric
                            // values fall through to ``.ok`` here —
                            // matching reference Tcl raises an error,
                            // tracked as a follow-up.
                            const parsed = parse_code_number(code.ptr, code.len);
                            if (parsed.ok) {
                                code_kind = .custom;
                                custom_code = parsed.n;
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
                    // Free a prior expander-minted value before storing
                    // this borrowed ``words[]`` slice
                    // (``-options {-errorcode A} -errorcode B``); a prior
                    // borrowed slot needs no release.
                    if (errorcode_obj != 0 and errorcode_owned) {
                        obj_ret_cleanup.tcl_obj_release(errorcode_obj);
                        errorcode_owned = false;
                    }
                    errorcode_obj = words[wi + 1];
                    wi += 1;
                    continue;
                }
                if (str_eq(wp, w.len, "-errorinfo") and wi + 1 < words.len) {
                    if (errorinfo_obj != 0 and errorinfo_owned) {
                        obj_ret_cleanup.tcl_obj_release(errorinfo_obj);
                        errorinfo_owned = false;
                    }
                    errorinfo_obj = words[wi + 1];
                    wi += 1;
                    continue;
                }
                if (str_eq(wp, w.len, "-options") and wi + 1 < words.len) {
                    // Expand the options DICT.  Standard slots
                    // (``-code`` / ``-level`` / ``-errorcode`` /
                    // ``-errorinfo``) are extracted into their
                    // dedicated tracking variables (so they pass
                    // through the normal ``-code`` / ``-level`` paths
                    // and the surrounding catch sees the right
                    // surface); a nested ``-options`` recurses; any
                    // other option accumulates into pending-extras.
                    // Multiple ``-options`` arguments to ``return``
                    // also accumulate (cmdMZ-return-2.20), and a dict
                    // with repeated / nested keys is applied in order
                    // and recursively flattened (cmdMZ-return-2.21).
                    const ctx = RetOptCtx{
                        .code_kind = &code_kind,
                        .custom_code = &custom_code,
                        .extra_levels = &extra_levels,
                        .level_explicit = &level_explicit,
                        .level_value = &level_value,
                        .errorcode_obj = &errorcode_obj,
                        .errorinfo_obj = &errorinfo_obj,
                        .errorcode_owned = &errorcode_owned,
                        .errorinfo_owned = &errorinfo_owned,
                    };
                    apply_return_options_dict(ctx, words[wi + 1], 0);
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
        // ``return -code error`` with no message word yields an
        // EMPTY-STRING result, not null — Tcl's ``catch`` then writes
        // ``""`` into its message variable.  Passing the raw null
        // ``result_obj`` here leaves ``state.error_msg`` at 0, so
        // ``catch {return -code error} m`` never sets ``m`` and the
        // following ``$m`` raises (proc-old-7.10).  Mirror the empty
        // ``error ""`` path with a fresh empty-string obj.
        const err_msg = if (result_obj != 0)
            result_obj
        else
            @import("../valtypes/tcl_obj.zig").obj_new_string(0, 0);
        catch_mod.tcl_cmd_error_full(err_msg, errorinfo_obj, errorcode_obj);
        // Tcl 9 distinguishes a body-level ``error msg`` (which
        // ``InterpProcNR2`` annotates with ``MakeProcError``) from
        // a ``return -code error msg`` (which goes through
        // ``TclUpdateReturnInfo`` and skips the procedure frame).
        // Our shortcut above goes straight to the error path; flag
        // it so the procedure-frame stamps (compiled epilogue +
        // interpreted dispatch) skip the frame.  proc-old-7.2's
        // expected ``::errorInfo`` carries no ``(procedure "tproc"
        // line N)`` line for this exact reason.
        catch_mod.state.return_via_error_code = 1;
        return result_mod.from_globals(0);
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
    // ``return -code break`` / ``return -code continue`` (default
    // ``-level 1``) unwind the proc body like a normal ``return`` — the
    // ``set_return`` below sets ``return_flag`` with ``return_level ==
    // 0`` and ``pending_return_code`` (3 / 4) was stamped above — but
    // ALSO arm the ``signal_break_flag`` / ``signal_continue_flag``
    // side-channel.  At the proc-dispatch / compiled-call absorb
    // boundary the pending code is translated into ``break_flag`` /
    // ``continue_flag`` for an enclosing loop to catch (interpreted:
    // ``apply_pending_return_code``; compiled: ``flow_take_return``).
    // When the loop-control ``return`` lands in a proc with no enclosing
    // loop the signal flag drives ``flow_check_signal_loop`` so the
    // compiled body returns out to its dispatcher, which re-stamps the
    // caller's flag.  The ``-level 0`` no-unwind form is handled above
    // and never reaches here.
    if (code_kind == .brk) {
        result_mod.set_signal_break();
    } else if (code_kind == .cont) {
        result_mod.set_signal_continue();
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
    // ``::errorCode`` from the optional 2nd / 3rd args.  Tcl 9
    // requires 1, 2 or 3 *user* args (plus the cmd name itself);
    // 0 or 4+ args raise ``wrong # args`` (error-5.1 / -5.2).
    if (words.len < 2 or words.len > 4) {
        stubs.raise("wrong # args: should be \"error message ?errorInfo? ?errorCode?\"");
        return result_mod.from_globals(0);
    }
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
        // ``catch`` evaluates its body — counts as one level
        // (mirrors C Tcl where catch's body dispatch goes through
        // ``TclNREvalObjEx`` which ``numLevels++``s).
        if (!interp.recursion_check_enter()) return result_mod.from_globals(0);
        defer interp.recursion_check_leave();
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
            // ``catch_options`` returns +1 owned; ``var_set`` retains
            // for the slot.  Release the local +1 share after the
            // store or every 4-arg catch leaks one dict per call.
            const opt_obj = catch_mod.catch_options();
            _ = frames.var_set(words[3], opt_obj);
            const obj_mod_catch = @import("../valtypes/tcl_obj.zig");
            obj_mod_catch.tcl_obj_release(opt_obj);
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

/// Parse a numeric literal accepting decimal (``42`` / ``-1`` /
/// ``+0``) and hex (``0x2A`` / ``0X2A``).  Used by ``try`` ``on code
/// …`` matching and the ``return -code N`` parser so both sides
/// recognise the full set Tcl 9 lets through — hex codes (error-
/// 10.11 / -10.12) AND negative custom codes (interp-26.x).
/// Returns (n, true) on success, (0, false) on failure.
fn parse_code_number(ptr: u32, len: u32) struct { n: i64, ok: bool } {
    if (len == 0 or ptr == 0) return .{ .n = 0, .ok = false };
    const cp: [*]const u8 = @ptrFromInt(ptr);
    // Handle an optional leading sign.  ``-N`` is a legal custom
    // completion code per ``return -code -1 ...`` (interp-26.x);
    // ``+N`` is accepted as well for symmetry with Tcl's number
    // parser.  A bare ``-`` / ``+`` with no following digits is
    // rejected by the ``ok = ki < len`` check below.
    var ki: u32 = 0;
    var negative = false;
    if (cp[0] == '-') {
        negative = true;
        ki = 1;
    } else if (cp[0] == '+') {
        ki = 1;
    }
    // Hex form ``0x…`` (no sign).  ``-0xNN`` would need separate
    // handling — Tcl 9 rejects it, so we do too.  Overflow on either
    // branch surfaces as ``ok = false`` rather than silently wrapping
    // (codex review on PR #452); the caller lets the canonical
    // "integer value too large" diagnostic stand.
    if (!negative and ki == 0 and len >= 3 and cp[0] == '0' and (cp[1] == 'x' or cp[1] == 'X')) {
        var n: i64 = 0;
        var i: u32 = 2;
        while (i < len) : (i += 1) {
            const c = cp[i];
            const d: i64 = if (c >= '0' and c <= '9')
                @as(i64, c - '0')
            else if (c >= 'a' and c <= 'f')
                @as(i64, c - 'a' + 10)
            else if (c >= 'A' and c <= 'F')
                @as(i64, c - 'A' + 10)
            else
                return .{ .n = 0, .ok = false };
            const m = @mulWithOverflow(n, @as(i64, 16));
            if (m[1] != 0) return .{ .n = 0, .ok = false };
            const a = @addWithOverflow(m[0], d);
            if (a[1] != 0) return .{ .n = 0, .ok = false };
            n = a[0];
        }
        return .{ .n = n, .ok = true };
    }
    var n: i64 = 0;
    if (ki >= len) return .{ .n = 0, .ok = false };
    while (ki < len) : (ki += 1) {
        const c = cp[ki];
        if (c < '0' or c > '9') return .{ .n = 0, .ok = false };
        const m = @mulWithOverflow(n, @as(i64, 10));
        if (m[1] != 0) return .{ .n = 0, .ok = false };
        const a = @addWithOverflow(m[0], @as(i64, c - '0'));
        if (a[1] != 0) return .{ .n = 0, .ok = false };
        n = a[0];
    }
    if (negative) {
        // ``-9223372036854775808`` (i64 min) can't be negated in i64
        // without overflow — Tcl 9 accepts this as a valid custom
        // completion code, so handle it explicitly.
        if (n == std.math.minInt(i64)) {
            // Already at the negative bound after the unsigned parse —
            // but actual parsing produced a positive value, so the
            // user's ``-n`` doesn't reach this case in practice.
            // Defensive bail.
            return .{ .n = 0, .ok = false };
        }
        n = -n;
    }
    return .{ .n = n, .ok = true };
}

/// Translate a ``try`` ``on code …`` code-word into its numeric
/// Tcl status equivalent: ``ok=0``, ``error=1``, ``return=2``,
/// ``break=3``, ``continue=4``.  Numeric literals (decimal or
/// hex ``0x...``) pass through.  Anything else returns -1 so the
/// caller treats the handler as "never matches" (real Tcl raises
/// ``bad completion code`` — follow-up, the existing tests don't
/// exercise it).
fn try_keyword_to_code(code_word: i32) i64 {
    const cs = obj_ensure_string(code_word);
    if (cs.len == 0) return -1;
    const cp: [*]const u8 = @ptrFromInt(cs.ptr);
    if (str_eq(cp, cs.len, "ok")) return 0;
    if (str_eq(cp, cs.len, "error")) return 1;
    if (str_eq(cp, cs.len, "return")) return 2;
    if (str_eq(cp, cs.len, "break")) return 3;
    if (str_eq(cp, cs.len, "continue")) return 4;
    const parsed = parse_code_number(cs.ptr, cs.len);
    if (!parsed.ok) return -1;
    return parsed.n;
}

/// Match a ``try ... trap pattern varlist body`` pattern against
/// the current error's ``::errorCode``.  The pattern is a list of
/// element prefixes (e.g. ``{FOO BAR}`` matches an errorCode whose
/// first two elements are ``FOO BAR``); an empty pattern matches
/// any error.  Mirrors Tcl 9 ``Tcl_TryObjCmd``'s ``MatchTrap`` —
/// element-wise prefix comparison using ``Tcl_StringMatch``-equivalent
/// (exact match here, no glob).
fn trap_pattern_matches(pattern_obj: i32) bool {
    const pat_s = obj_ensure_string(pattern_obj);
    const pat_n = rt.list_count_elements(pat_s.ptr, pat_s.len);
    if (pat_n == 0) return true; // empty pattern → catch-all
    // Probe ``::errorCode`` for the error's code list.
    const obj_mod = @import("../valtypes/tcl_obj.zig");
    const tcl_ns_mod = @import("../interp/tcl_ns.zig");
    const ec_name = obj_mod.obj_new_string_copy(@intFromPtr("::errorCode".ptr), 11);
    defer obj_mod.tcl_obj_release(ec_name);
    const ec_val = tcl_ns_mod.global_get(ec_name);
    if (ec_val == 0) return false;
    const ec_s = obj_ensure_string(ec_val);
    const ec_n = rt.list_count_elements(ec_s.ptr, ec_s.len);
    if (pat_n > ec_n) return false;
    // Compare element-by-element (byte equality).
    var i: i64 = 0;
    while (i < pat_n) : (i += 1) {
        const pe = rt.list_element_at(pat_s.ptr, pat_s.len, i);
        const ee = rt.list_element_at(ec_s.ptr, ec_s.len, i);
        if (pe.len != ee.len) return false;
        if (pe.len == 0) continue;
        const pp: [*]const u8 = @ptrFromInt(pat_s.ptr + pe.start);
        const ep: [*]const u8 = @ptrFromInt(ec_s.ptr + ee.start);
        var k: u32 = 0;
        while (k < pe.len) : (k += 1) {
            if (pp[k] != ep[k]) return false;
        }
    }
    return true;
}

// ``try body ?on code varlist handler? ... ?finally body?``
fn eval_try(words: []const i32) result_mod.InterpResult {
    if (words.len < 2) return result_mod.from_globals(obj_new_string(0, 0));
    const interp = @import("../interp/tcl_interp.zig");

    rt.catch_enter();
    // Hygiene: drop any ``during_options`` leaked from a prior ``try``
    // whose finally raised but whose options dict was never read
    // (``eval_catch`` only consumes the slot for the 4-arg
    // ``catch BODY result opts`` form, so a 2-/3-arg catch leaves it
    // set).  Clearing here keeps this try's body/handler options
    // snapshot from inheriting an unrelated chain — the body's own
    // nested ``try ... finally`` re-arms the slot if it needs to.
    if (catch_mod.state.during_options != 0) {
        const obj_mod_entry = @import("../valtypes/tcl_obj.zig");
        obj_mod_entry.tcl_obj_release(catch_mod.state.during_options);
        catch_mod.state.during_options = 0;
    }
    const body_s = obj_ensure_string(words[1]);
    const body_res = interp.eval_script(body_s.ptr, body_s.len);
    rt.catch_set_ok_result(body_res);
    // Order matters: ``catch_leave`` snapshots ``last_catch_had_error``
    // that ``catch_result`` reads.  See ``eval_catch`` above.
    const code_obj = rt.catch_leave();
    const catch_val = rt.catch_result();
    // ``body_code`` is the numeric Tcl status code returned by the
    // body: 0=OK, 1=ERROR, 2=RETURN, 3=BREAK, 4=CONTINUE, ≥5 custom.
    // Used to match ``on code`` handlers (error-9.x covers each).
    const body_code: i64 = rt.obj_get_int(code_obj);
    const had_error: bool = body_code == 1;

    // Snapshot the body's return options so ``try`` can re-raise
    // them when no handler matches.  ``catch_leave`` consumed the
    // pending slots into ``last_return_*``; capture those now so a
    // subsequent handler body (which uses ``set`` / nested ``catch``
    // etc.) can't overwrite them before we re-promote.  These match
    // the options dict the enclosing catch would have produced for
    // the body alone (error-15.x .2 variants check ``catch $script``
    // ≡ ``catch {try $script}`` options dict).  We keep the snapshot
    // live and the live ``last_return_extras`` slot intact so any
    // ``catch_options`` query a matched handler makes still sees the
    // body's options (the handler-body's ``set $y [catch_options]``
    // is the common shape error-15.10 exercises).
    const saved_return_code: i64 = catch_mod.state.last_return_code;
    const saved_return_level: u32 = catch_mod.state.last_return_level;
    const saved_return_extras: i32 = catch_mod.state.last_return_extras;
    if (saved_return_extras != 0) {
        const obj_mod_try = @import("../valtypes/tcl_obj.zig");
        obj_mod_try.tcl_obj_retain(saved_return_extras);
    }

    var final_result: i32 = if (had_error) catch_val else body_res;
    var error_raised = had_error;
    var handled = false;
    var finally_body: i32 = 0;
    // ``pending_chain`` is true after a handler matched but its body
    // was the single-byte token ``-``: the next handler's body runs
    // unconditionally (no further code/pattern match), with the
    // ORIGINAL varlist binding preserved.  Tcl 9 TIP 329 semantics:
    // ``try {…} on ok {x} - on error {} {…}`` — the ``on ok {x} -``
    // arm matched on OK, but its dash body means "use the next
    // handler's body".  Tested by error-19.1 / -19.2 / -19.3 / -19.4
    // / -19.5.  The chained handler may itself have a ``-`` body,
    // which keeps the chain going until a real body is found.
    var pending_chain: bool = false;

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

        // Parse a handler word: ``on code varlist handler`` or
        // ``trap pattern varlist handler``.  Decide whether the
        // handler matches (or whether we're in a chain), then run
        // (or skip, or chain again) the body.
        const is_on = kw.len == 2 and kp[0] == 'o' and kp[1] == 'n';
        const is_trap = kw.len == 4 and kp[0] == 't' and kp[1] == 'r' and kp[2] == 'a' and kp[3] == 'p';
        if (is_on or is_trap) {
            wi += 1;
            if (wi + 2 >= words.len) break;
            const guard_word = words[wi];
            wi += 1;
            const varlist = words[wi];
            wi += 1;
            const handler = words[wi];
            wi += 1;
            // Compute whether THIS handler matches the body's outcome,
            // independent of any pending chain (a chain doesn't check
            // matching — it just takes the next body).
            var this_matches: bool = false;
            if (is_on) {
                this_matches = !handled and try_keyword_to_code(guard_word) == body_code;
            } else {
                this_matches = !handled and had_error and trap_pattern_matches(guard_word);
            }
            const run_body = pending_chain or this_matches;
            if (run_body) {
                // Bind the handler's varlist when *this* handler is
                // the one matching (a chain keeps the previous
                // handler's bindings).  Skip binding when we're
                // continuing a chain.
                if (this_matches and !pending_chain) {
                    const vl_s = obj_ensure_string(varlist);
                    const vl_n = rt.list_count_elements(vl_s.ptr, vl_s.len);
                    if (vl_n >= 1) {
                        const v0 = rt.list_element_at(vl_s.ptr, vl_s.len, 0);
                        if (v0.len > 0) {
                            const vn = rt.obj_new_string_copy(vl_s.ptr + v0.start, v0.len);
                            _ = frames.var_set(vn, catch_val);
                            // Release the freshly-allocated var-name
                            // TclObj — ``var_set`` reads its bytes
                            // without taking ownership, so the +1 share
                            // returned by ``obj_new_string_copy`` would
                            // otherwise leak per matched handler.
                            const obj_mod_try = @import("../valtypes/tcl_obj.zig");
                            obj_mod_try.tcl_obj_release(vn);
                        }
                    }
                    if (vl_n >= 2) {
                        const v1 = rt.list_element_at(vl_s.ptr, vl_s.len, 1);
                        if (v1.len > 0) {
                            const vn = rt.obj_new_string_copy(vl_s.ptr + v1.start, v1.len);
                            // ``catch_options`` returns +1 owned; ``var_set``
                            // retains for the slot.  Release the local +1
                            // and the freshly minted var-name obj or each
                            // matched handler with an options-var arg leaks
                            // one dict + one name TclObj per execution.
                            const opt_obj = catch_mod.catch_options();
                            _ = frames.var_set(vn, opt_obj);
                            const obj_mod_try = @import("../valtypes/tcl_obj.zig");
                            obj_mod_try.tcl_obj_release(opt_obj);
                            obj_mod_try.tcl_obj_release(vn);
                        }
                    }
                }
                // Inspect handler body: is it the chain sentinel ``-``?
                const hb_s = obj_ensure_string(handler);
                const hp: [*]const u8 = if (hb_s.len > 0) @ptrFromInt(hb_s.ptr) else undefined;
                if (hb_s.len == 1 and hp[0] == '-') {
                    // Chain to the next handler; keep the current
                    // bindings (or, if we just took a chain ourselves,
                    // the original matcher's bindings).
                    pending_chain = true;
                    continue;
                }
                // Snapshot the body's (or prior chain's) effective
                // options BEFORE the handler runs — a throwing handler
                // clobbers ::errorInfo/::errorCode, so the body's error
                // metadata must be captured now.  C Tcl's TryPostHandler
                // (tclCmdMZ.c) chains these under ``-during`` when the
                // handler itself raises (error-16.8/.9, -18.10), and
                // discards them — the handler's options replace the
                // body's wholesale — when the handler completes without
                // raising (error-16.10, -18.9).  ``catch_options`` also
                // folds in (and clears) any pending ``during_options``.
                const obj_mod_h = @import("../valtypes/tcl_obj.zig");
                const body_opts = catch_mod.catch_options();
                final_result = interp.eval_script(hb_s.ptr, hb_s.len);
                error_raised = false;
                handled = true;
                pending_chain = false;
                if (catch_mod.state.error_flag != 0) {
                    // Tcl 9 bytecode-compiles ``try``, so it never appears
                    // in the traceback — suppress the outer ``invoked from
                    // within "try ..."`` frame (error-15.10.x check this).
                    catch_mod.state.transparent_error = 1;
                    if (catch_mod.state.return_via_error_code == 0) {
                        // Handler raised a *direct* error (``throw`` /
                        // ``error``): C Tcl's TryPostHandler sees TCL_ERROR
                        // and calls During(), chaining the body's options
                        // under ``-during``.  Force the option slots to an
                        // ERROR outcome so ``catch_options`` emits
                        // -errorcode / -errorinfo / -code 1 even when the
                        // body was OK (error-16.8's ``on ok`` handler that
                        // throws) — then attach the captured body options.
                        catch_mod.state.last_catch_had_error = 1;
                        catch_mod.state.last_return_code = 1;
                        catch_mod.state.last_return_level = 0;
                        if (catch_mod.state.last_return_extras != 0) {
                            obj_mod_h.tcl_obj_release(catch_mod.state.last_return_extras);
                            catch_mod.state.last_return_extras = 0;
                        }
                        // ``catch_options`` consumes this slot exactly once
                        // — the finally's during_snap below, or the
                        // enclosing catch when there is no finally
                        // (error-16.8/.9, -18.10).  Release any chain the
                        // handler body itself left pending (a nested
                        // ``try ... finally {throw}``): C Tcl's During
                        // overwrites the ``-during`` key with the outer
                        // body's options, so the inner chain is dropped.
                        if (catch_mod.state.during_options != 0) {
                            obj_mod_h.tcl_obj_release(catch_mod.state.during_options);
                        }
                        catch_mod.state.during_options = body_opts;
                    } else {
                        // Handler re-raised via ``return -code error`` /
                        // ``return -options $opts $msg``: C Tcl evaluates
                        // this as TCL_RETURN (the level decrement happens
                        // at the eval boundary), so TryPostHandler takes
                        // the no-During branch and the handler's return
                        // options replace the body's verbatim — error-15.10
                        // checks ``catch {try …}`` ≡ ``catch $script``.
                        // ``eval_return`` already armed pending_return_* with
                        // the re-raised dict; let the enclosing catch_leave
                        // snapshot them.  Drop the body snapshot, no chain.
                        if (body_opts != 0) obj_mod_h.tcl_obj_release(body_opts);
                    }
                } else {
                    // Handler completed without raising.  Its options
                    // replace the body's (no ``-during``); drop the
                    // snapshot.  For a clean OK completion reset the option
                    // slots so ``catch_options`` reports the handler's
                    // code 0 — not the body's sticky error (error-18.9
                    // checks ``-during -code 0`` after an ``on error``
                    // handler swallowed the body error).  A return / break
                    // / continue handler keeps its flow signal and pending
                    // option slots untouched so it still propagates.
                    if (body_opts != 0) obj_mod_h.tcl_obj_release(body_opts);
                    if (catch_mod.state.return_flag == 0 and
                        catch_mod.state.break_flag == 0 and
                        catch_mod.state.continue_flag == 0)
                    {
                        catch_mod.state.last_catch_had_error = 0;
                        catch_mod.state.last_return_code = 0;
                        catch_mod.state.last_return_level = 0;
                        if (catch_mod.state.last_return_extras != 0) {
                            obj_mod_h.tcl_obj_release(catch_mod.state.last_return_extras);
                            catch_mod.state.last_return_extras = 0;
                        }
                    }
                }
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
        // Snapshot the body/handler's effective options dict BEFORE
        // running the finally.  Tcl 9 ``TryPostFinally`` (tclCmdMZ.c)
        // builds the ``-during`` field of the finally's options when
        // the finally raises an error AND a prior body/handler had
        // also raised.  We capture unconditionally — including OK
        // outcomes — so error-18.8 / -18.9 (body=OK, finally=error)
        // see ``-during -code 0`` as well.  The snapshot is owned by
        // this scope and either consumed into ``state.during_options``
        // for the catch's options dict, or released on the OK path.
        const obj_mod_try = @import("../valtypes/tcl_obj.zig");
        const during_snap = catch_mod.catch_options();
        const snap = result_mod.flow_save_and_clear();
        rt.catch_enter();
        const fb_s = obj_ensure_string(finally_body);
        const fb_result = interp.eval_script(fb_s.ptr, fb_s.len);
        rt.catch_set_ok_result(fb_result);
        // ``catch_leave`` before ``catch_result`` — see ``eval_catch``.
        const finally_code = rt.catch_leave();
        const fb_val = rt.catch_result();
        const fb_code = rt.obj_get_int(finally_code);
        if (fb_code == 1) {
            // Finally raised an error — propagate it via the standard
            // ``tcl_cmd_error_full`` path so refcount + globals are
            // managed consistently with the throw/error code.  Read
            // back ``::errorCode`` first so the finally's class (set
            // by ``throw CODE MSG``) survives — passing it as the
            // ``code`` arg keeps ``stamp_error_globals`` from resetting
            // it to the default ``NONE``.
            const ec_name = rt.obj_new_string_copy(@intFromPtr("::errorCode".ptr), 11);
            const tcl_ns_mod = @import("../interp/tcl_ns.zig");
            const ec_obj = tcl_ns_mod.global_get(ec_name);
            obj_mod_try.tcl_obj_release(ec_name);
            catch_mod.tcl_cmd_error_full(fb_val, 0, ec_obj);
            if (ec_obj != 0) obj_mod_try.tcl_obj_release(ec_obj);
            // Stash the prior chain under ``-during`` so the catch's
            // options dict reflects which earlier completion was
            // overridden by the finally's error.  Tcl 9
            // ``TryPostFinal`` mirrors this with
            // ``During(interp, ERROR, options, ...)`` — error-16.21 /
            // -18.7 / -18.10 check this.  ``catch_options`` consumes
            // the slot exactly once.
            if (during_snap != 0) {
                catch_mod.state.during_options = during_snap;
            }
            return result_mod.from_globals(fb_result);
        }
        // Finally exited non-OK without an error (TCL_RETURN/BREAK/
        // CONTINUE) OR cleanly (TCL_OK) — the during snapshot is no
        // longer relevant since the finally itself produced the new
        // outcome; release it.  The flow signal (if any) was already
        // absorbed by the inner catch_leave; the fall-through below
        // either re-raises a pre-existing handler error, or restores
        // the saved signal so a return / break / continue from the
        // finally propagates back to the caller's enclosing
        // proc / loop.
        if (during_snap != 0) {
            obj_mod_try.tcl_obj_release(during_snap);
        }
        // If finally set a return/break/continue signal, propagate it.
        const fb_ir = result_mod.snapshot(fb_result);
        if (fb_ir.code == .RETURN or fb_ir.code == .BREAK or fb_ir.code == .CONTINUE) {
            return result_mod.from_globals(fb_result);
        }
        // Finally completed normally: restore signals from after handlers ran.
        result_mod.flow_restore(snap);
    }

    if (error_raised and !handled) {
        // Re-raise the body's error without resetting ``::errorInfo``
        // (which still carries the body's traceback from when
        // ``tcl_cmd_error`` first fired).  ``tcl_cmd_error`` would
        // stamp ``::errorInfo`` back to just the message and erase
        // the body's frames; setting the flag + msg directly keeps
        // the existing traceback intact.  Mirrors Tcl 9 bytecode-
        // compiled ``try``: when no handler matches, the error
        // propagates transparently through ``try`` without rewriting
        // the trace (error-15.x .1.1.x checks).
        catch_mod.state.error_flag = 1;
        const obj_mod_try = @import("../valtypes/tcl_obj.zig");
        if (catch_val != 0) obj_mod_try.tcl_obj_retain(catch_val);
        if (catch_mod.state.error_msg != 0 and catch_mod.state.error_msg != catch_val) {
            obj_mod_try.tcl_obj_release(catch_mod.state.error_msg);
        }
        catch_mod.state.error_msg = catch_val;
        // Also suppress the outer ``invoked from within "try $script"``
        // frame the enclosing eval_script would add — ``try`` is
        // transparent in the traceback.
        catch_mod.state.transparent_error = 1;
    }
    // When no handler matched, ``try`` should propagate the body's
    // return options (e.g. ``-bar soom`` from
    // ``return -level 0 -code 0 -bar soom foo`` or ``-level 1`` from
    // ``return -level 1 -code 1 foo``) to the enclosing catch so its
    // options dict mirrors the catch-without-try form.  We snapshotted
    // the body's options at the top of this function; re-attach to
    // ``pending_return_*`` so the enclosing catch's ``catch_leave``
    // picks them up (error-15.x .2 / .1.1.x variants check
    // ``catch $script`` ≡ ``catch {try $script}`` options dicts).
    if (!handled) {
        catch_mod.state.pending_return_armed = 1;
        catch_mod.state.pending_return_code = saved_return_code;
        catch_mod.state.pending_return_level = saved_return_level;
        if (saved_return_extras != 0) {
            catch_mod.state.pending_return_extras = saved_return_extras;
        }
    } else if (saved_return_extras != 0) {
        // Handled path — drop the saved extras retain we took above
        // so it doesn't leak.
        const obj_mod_try = @import("../valtypes/tcl_obj.zig");
        obj_mod_try.tcl_obj_release(saved_return_extras);
    }
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
    const obj_mod_time = @import("../valtypes/tcl_obj.zig");

    const count: i64 = if (words.len >= 3) rt.obj_get_int(words[2]) else 1;
    const body_s = obj_ensure_string(words[1]);

    const start_us = clicks_us();
    var i: i64 = 0;
    var last: i32 = 0;
    while (i < count) : (i += 1) {
        // Each iteration's eval_script returns +1 owned.  Release
        // the previous iteration's result before overwriting so a
        // ``time { ... } 1000`` body doesn't leak 999 result objs.
        if (last != 0) obj_mod_time.tcl_obj_release(last);
        last = interp.eval_script(body_s.ptr, body_s.len);
        const ir = result_mod.snapshot(last);
        if (ir.code == .ERROR or ir.code == .RETURN) return result_mod.from_globals(last);
    }
    // Loop completed normally — release the final body result
    // (we don't propagate it; the summary string is built fresh).
    if (last != 0) obj_mod_time.tcl_obj_release(last);
    const end_us = clicks_us();
    const per_iter = if (count > 0) @divTrunc(end_us - start_us, count) else 0;

    // Build "<N> microseconds per iteration".  ``obj_new_int`` mints
    // a fresh +1 owned that we use only for its byte form; release
    // immediately after the memcpy so the int obj doesn't outlive
    // the function.
    const pi_obj = rt.obj_new_int(per_iter);
    const pi_s = rt.obj_ensure_string(pi_obj);
    const suffix = " microseconds per iteration";
    const total: u32 = @intCast(pi_s.len + suffix.len);
    const buf = rt.alloc(total);
    if (buf == 0) {
        obj_mod_time.tcl_obj_release(pi_obj);
        return result_mod.from_globals(rt.obj_new_string(0, 0));
    }
    rt.memcpy(buf, pi_s.ptr, pi_s.len);
    var k: u32 = 0;
    while (k < suffix.len) : (k += 1) {
        const d: [*]u8 = @ptrFromInt(buf + pi_s.len + k);
        d[0] = suffix[k];
    }
    obj_mod_time.tcl_obj_release(pi_obj);
    return result_mod.from_globals(rt.obj_new_string_take(buf, total, total));
}

const TIMERATE_USAGE =
    "wrong # args: should be \"timerate ?-direct? ?-calibrate? " ++
    "?-overhead double? command ?time ?max-count??\"";

/// Raise ``expected integer but got "X"`` (or the floating-point
/// variant) for a bad ``timerate`` numeric argument.
fn timerate_expected(bytes_ptr: u32, bytes_len: u32, as_float: bool) void {
    const int_prefix: []const u8 = "expected integer but got \"";
    const flt_prefix: []const u8 = "expected floating-point number but got \"";
    const prefix = if (as_float) flt_prefix else int_prefix;
    const total: u32 = @intCast(prefix.len + bytes_len + 1);
    const buf = rt.alloc(total);
    if (buf == 0) {
        stubs.raise(prefix);
        return;
    }
    const dst: [*]u8 = @ptrFromInt(buf);
    var off: u32 = 0;
    for (prefix) |c| {
        dst[off] = c;
        off += 1;
    }
    if (bytes_len > 0) {
        const src: [*]const u8 = @ptrFromInt(bytes_ptr);
        var k: u32 = 0;
        while (k < bytes_len) : (k += 1) {
            dst[off] = src[k];
            off += 1;
        }
    }
    dst[off] = '"';
    off += 1;
    catch_mod.tcl_cmd_error(rt.obj_new_string_take(buf, off, total));
}

/// Build the 8-element ``timerate`` result list as a space-joined
/// string: ``<µs/#> µs/# <count> # <#/sec> #/sec <net-ms> net-ms``.
fn timerate_result(usec_in: i64, count: i64) i32 {
    var buf: [192]u8 = undefined;
    // ``µ`` is U+00B5 → UTF-8 0xC2 0xB5.
    const mu = "\xC2\xB5";
    var s: []u8 = undefined;
    if (count <= 0) {
        s = std.fmt.bufPrint(&buf, "0 {s}s/# 0 # 0 #/sec 0 net-ms", .{mu}) catch
            return obj_new_string(0, 0);
    } else {
        const usec: i64 = if (usec_in < 0) 0 else usec_in;
        const per_iter = @divTrunc(usec, count);
        const usec_nz: i64 = if (usec == 0) 1 else usec;
        const per_sec: i64 = @intCast(@divTrunc(@as(i128, count) * 1_000_000, usec_nz));
        if (usec >= 1) {
            // net-ms = usec/1000 with exactly three fractional digits.
            const frac = @mod(usec, 1000);
            s = std.fmt.bufPrint(
                &buf,
                "{d} {s}s/# {d} # {d} #/sec {d}.{d}{d}{d} net-ms",
                .{
                    per_iter,              mu,                   count,                          per_sec,
                    @divTrunc(usec, 1000), @divTrunc(frac, 100), @divTrunc(@mod(frac, 100), 10), @mod(frac, 10),
                },
            ) catch return obj_new_string(0, 0);
        } else {
            s = std.fmt.bufPrint(
                &buf,
                "{d} {s}s/# {d} # {d} #/sec 0 net-ms",
                .{ per_iter, mu, count, per_sec },
            ) catch return obj_new_string(0, 0);
        }
    }
    const out = rt.alloc(@intCast(s.len));
    if (out == 0) return obj_new_string(0, 0);
    rt.memcpy(out, @intFromPtr(s.ptr), @intCast(s.len));
    return rt.obj_new_string_take(out, @intCast(s.len), @intCast(s.len));
}

// ``timerate ?-direct? ?-calibrate? ?-overhead double? command ?time
// ?max-count??`` — run *command* repeatedly until *time* ms elapse (or
// *max-count* iterations) and report a timing summary.  Mirrors
// ``Tcl_TimeRateObjCmd``; the adaptive batching / calibration of the C
// version is collapsed to a straight loop (the ``-direct`` /
// ``-calibrate`` flags are accepted but don't change the measurement).
fn eval_timerate(words: []const i32) result_mod.InterpResult {
    const interp = @import("../interp/tcl_interp.zig");
    const om = @import("../valtypes/tcl_obj.zig");

    var i: u32 = 1;
    var overhead: f64 = 0;
    while (i < words.len) : (i += 1) {
        const w = obj_ensure_string(words[i]);
        if (w.len == 0) break;
        const wp: [*]const u8 = @ptrFromInt(w.ptr);
        if (wp[0] != '-') break;
        if (str_eq(wp, w.len, "-direct") or str_eq(wp, w.len, "-calibrate")) continue;
        if (str_eq(wp, w.len, "--")) {
            i += 1;
            break;
        }
        if (str_eq(wp, w.len, "-overhead")) {
            i += 1;
            if (i >= words.len) {
                stubs.raise(TIMERATE_USAGE);
                return result_mod.from_globals(obj_new_int(0));
            }
            const ov_s = obj_ensure_string(words[i]);
            if (om.try_parse_float(ov_s.ptr, ov_s.len)) |f| {
                overhead = f;
            } else if (om.try_parse_int(ov_s.ptr, ov_s.len)) |n| {
                overhead = @floatFromInt(n);
            } else {
                timerate_expected(ov_s.ptr, ov_s.len, true);
                return result_mod.from_globals(obj_new_int(0));
            }
            continue;
        }
        break;
    }

    // Positional: command ?time ?max-count??.  None, or more than three,
    // is a usage error (checked before the numeric args are parsed).
    if (i >= words.len or i + 3 < words.len) {
        stubs.raise(TIMERATE_USAGE);
        return result_mod.from_globals(obj_new_int(0));
    }
    const cmd = words[i];
    i += 1;
    var maxms: i64 = 1000;
    var maxcnt: i64 = std.math.maxInt(i64);
    if (i < words.len) {
        const t_s = obj_ensure_string(words[i]);
        const t = om.try_parse_int(t_s.ptr, t_s.len) orelse {
            timerate_expected(t_s.ptr, t_s.len, false);
            return result_mod.from_globals(obj_new_int(0));
        };
        maxms = t;
        i += 1;
        if (i < words.len) {
            const c_s = obj_ensure_string(words[i]);
            const v = om.try_parse_int(c_s.ptr, c_s.len) orelse {
                timerate_expected(c_s.ptr, c_s.len, false);
                return result_mod.from_globals(obj_new_int(0));
            };
            maxcnt = if (v > 0) v else 0;
        }
    }

    const body_s = obj_ensure_string(cmd);
    const maxms_us: i64 = if (maxms > 0) maxms * 1000 else 0;
    var count: i64 = 0;
    const start_us = clicks_us();
    while (count < maxcnt) {
        const r = interp.eval_script(body_s.ptr, body_s.len);
        const ir = result_mod.snapshot(r);
        // ``error`` / ``return`` propagate out of timerate verbatim
        // (errorInfo is already stamped by eval_script).
        if (ir.code == .ERROR or ir.code == .RETURN) {
            return result_mod.from_globals(r);
        }
        count += 1;
        if (r != 0) om.tcl_obj_release(r);
        // ``break`` ends the measurement after counting this iteration;
        // ``continue`` / ``ok`` keep going.
        if (ir.code == .BREAK) break;
        const now_us = clicks_us();
        if (now_us - start_us >= maxms_us) break;
    }
    const end_us = clicks_us();
    var usec: i64 = end_us - start_us;
    if (overhead > 0 and count > 0) {
        const cur: i64 = @intFromFloat(overhead * @as(f64, @floatFromInt(count)));
        usec = if (usec > cur) usec - cur else 0;
    }
    return result_mod.from_globals(timerate_result(usec, count));
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
    .{ .name = "timerate", .arity_min = 1, .arity_max = null, .handler = &eval_timerate },
};
