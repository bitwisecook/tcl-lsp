// Command: info — introspection into the interpreter state.
//
// Subcommands implemented:
//   info exists varName  — check if variable is defined (local or global)
//   info body procName   — return the body of a registered proc
//   info args procName   — return the parameter list of a registered proc
//
// Unimplemented subcommands (future work):
//   info commands ?pattern?   — list built-in + registered commands
//   info procs    ?pattern?   — list registered procedures
//   info level                — current frame depth
//   info vars / info locals / info globals
// info_dispatch() returns an empty string for any subcommand not in the
// list above; this is an explicit NOP rather than an error so code using
// unsupported introspection degrades gracefully in the WASM sandbox.
//
// Operates on frames (tcl_frames.zig) and proc registry (tcl_procs.zig).
// Callable from both the interpreter dispatch and WASM codegen imports.

const obj = @import("tcl_obj.zig");
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_int = obj.obj_new_int;
const obj_new_string = obj.obj_new_string;

const frames = @import("tcl_frames.zig");
const procs = @import("tcl_procs.zig");

fn str_eq(a: [*]const u8, alen: u32, comptime b: []const u8) bool {
    if (alen != b.len) return false;
    inline for (0..b.len) |i| {
        if (a[i] != b[i]) return false;
    }
    return true;
}

/// info exists varName — returns 1 if variable is defined, 0 otherwise.
/// Checks current frame locals first, then globals.
pub export fn info_exists(name: i32) i32 {
    return frames.var_exists(name);
}

/// info body procName — returns the body of an interpreted proc, or empty string.
pub export fn info_body(name: i32) i32 {
    const bucket = procs.proc_lookup(name);
    if (bucket == 0) return obj_new_string(0, 0);
    return procs.proc_get_body(bucket);
}

/// info args procName — returns the parameter list of a proc, or empty string.
pub export fn info_args(name: i32) i32 {
    const bucket = procs.proc_lookup(name);
    if (bucket == 0) return obj_new_string(0, 0);
    return procs.proc_get_params(bucket);
}

/// info complete script — returns 1 if the script has matched
/// braces / brackets / quotes so ``eval`` would accept it, 0
/// otherwise.  This is a structural sanity check used by tcltest
/// and other harnesses to validate user-supplied scripts before
/// evaluating them.  We walk the string once, tracking brace and
/// bracket nesting depth and honouring ``\`` escapes.
///
/// Brackets inside braced text (``{a [b} c]``) do NOT count —
/// brace-quoted words treat ``[`` as a literal character, not a
/// command-substitution marker.  So a script like ``"{[}"`` is
/// structurally complete: the outer ``[`` / ``]`` are consumed by
/// the braced word, which is itself balanced.  Only brackets
/// outside brace depth 0 affect completeness.  Similarly quotes
/// are literal inside braces.
pub fn info_complete(script: i32) i32 {
    const s = obj_ensure_string(script);
    if (s.len == 0) return obj_new_int(1);
    const sp: [*]const u8 = @ptrFromInt(s.ptr);
    var brace: i32 = 0;
    var bracket: i32 = 0;
    var in_quote: bool = false;
    var i: u32 = 0;
    while (i < s.len) : (i += 1) {
        const c = sp[i];
        if (c == '\\' and i + 1 < s.len) {
            i += 1;
            continue;
        }
        if (brace > 0) {
            // Inside braces: only ``{`` / ``}`` (and escaped chars)
            // affect completeness.  Brackets and quotes are
            // literal bytes.
            switch (c) {
                '{' => brace += 1,
                '}' => brace -= 1,
                else => {},
            }
            continue;
        }
        if (in_quote) {
            if (c == '"') in_quote = false;
            continue;
        }
        switch (c) {
            '{' => brace += 1,
            '}' => brace -= 1,
            '[' => bracket += 1,
            ']' => bracket -= 1,
            '"' => in_quote = true,
            else => {},
        }
        // A stray ``}`` or ``]`` (depth going negative) is a
        // structural error that can't be rebalanced — scripts
        // like ``}{`` or ``][`` would zero-sum at end-of-string
        // but are malformed.  Bail early with incomplete=0.
        if (brace < 0 or bracket < 0) return obj_new_int(0);
    }
    if (brace == 0 and bracket == 0 and !in_quote) return obj_new_int(1);
    return obj_new_int(0);
}

/// info nameofexecutable — returns a placeholder "tcl_runtime.wasm"
/// so callers that use it for discovery get a non-empty string.
pub fn info_nameofexecutable() i32 {
    return obj.obj_new_string_copy(@intFromPtr("tcl_runtime.wasm".ptr), 16);
}

/// info tclversion / info patchlevel — Tcl compatibility hooks.
pub fn info_tclversion() i32 {
    return obj.obj_new_string_copy(@intFromPtr("8.6".ptr), 3);
}

pub fn info_patchlevel() i32 {
    return obj.obj_new_string_copy(@intFromPtr("8.6.15".ptr), 6);
}

/// info hostname — return "wasm" (no real hostname inside the sandbox).
pub fn info_hostname() i32 {
    return obj.obj_new_string_copy(@intFromPtr("wasm".ptr), 4);
}

/// info commands — return empty list (we don't track command names).
pub fn info_commands() i32 {
    return obj_new_string(0, 0);
}

/// Dispatch for 'info' command. words[0] = "info", words[1] = subcommand, ...
/// Called by the interpreter's eval_command.
pub export fn info_dispatch(subcmd: i32, arg: i32) i32 {
    const sub = obj_ensure_string(subcmd);
    if (sub.len == 0) return obj_new_string(0, 0);
    const sp: [*]const u8 = @ptrFromInt(sub.ptr);

    if (str_eq(sp, sub.len, "exists")) {
        return info_exists(arg);
    }
    if (str_eq(sp, sub.len, "body")) {
        return info_body(arg);
    }
    if (str_eq(sp, sub.len, "args")) {
        return info_args(arg);
    }
    if (str_eq(sp, sub.len, "complete")) {
        return info_complete(arg);
    }
    if (str_eq(sp, sub.len, "nameofexecutable")) {
        return info_nameofexecutable();
    }
    if (str_eq(sp, sub.len, "tclversion")) {
        return info_tclversion();
    }
    if (str_eq(sp, sub.len, "patchlevel")) {
        return info_patchlevel();
    }
    if (str_eq(sp, sub.len, "hostname")) {
        return info_hostname();
    }
    if (str_eq(sp, sub.len, "commands")) {
        return info_commands();
    }
    // Unimplemented subcommands return empty string
    return obj_new_string(0, 0);
}
