// Active-vwait tracker.
//
// ``vwait`` calls ``begin(varName)`` before draining the scheduler
// and ``end()`` when the var has been written.  In between, every
// global-variable write checks :func:`note_var_write` to see if the
// written name matches one of the active ``vwait`` targets and, if
// so, sets the matching frame's ``done`` bit.  The drain loop
// breaks when ``current().done`` becomes true.
//
// Nesting is supported: if a scheduled handler calls ``vwait`` on a
// different variable, that's a new frame on top of the stack.  Only
// the top frame's variable is checked on writes — matching real Tcl
// where nested vwaits compose by stacking.  When a write triggers
// the OUTER variable while we're inside an inner vwait, the outer
// frame's ``done`` bit is set too (we walk the whole stack), so the
// outer drain notices on the next iteration after the inner returns.
//
// Stack capacity is fixed (8 levels) to keep the storage trivially
// linear.  Real Tcl scripts rarely nest vwaits more than 2 deep;
// 8 is generous and we trap on overflow with a clear error.

const obj = @import("../valtypes/tcl_obj.zig");

const MAX_VWAIT_DEPTH: u32 = 8;

pub const Frame = struct {
    var_name_ptr: u32,
    var_name_len: u32,
    done: bool,
};

pub const Stack = struct {
    frames: [MAX_VWAIT_DEPTH]Frame = undefined,
    len: u32 = 0,

    pub fn empty(self: *const Stack) bool {
        return self.len == 0;
    }

    /// Push a vwait frame for ``varName``.  Returns true on success,
    /// false on overflow.  ``var_name`` is the post-substitution
    /// scalar name; the bytes are referenced by pointer (no copy)
    /// because the caller's TclObj outlives the vwait drain.
    pub fn push(self: *Stack, var_name_ptr: u32, var_name_len: u32) bool {
        if (self.len >= MAX_VWAIT_DEPTH) return false;
        self.frames[self.len] = .{
            .var_name_ptr = var_name_ptr,
            .var_name_len = var_name_len,
            .done = false,
        };
        self.len += 1;
        return true;
    }

    pub fn pop(self: *Stack) void {
        if (self.len > 0) self.len -= 1;
    }

    pub fn current(self: *Stack) ?*Frame {
        if (self.len == 0) return null;
        return &self.frames[self.len - 1];
    }

    /// Strip a leading ``::`` (real Tcl normalises ``vwait foo`` and
    /// ``vwait ::foo`` to the same target).  Returns the post-strip
    /// span.
    fn strip_global(ptr: u32, len: u32) struct { ptr: u32, len: u32 } {
        if (len >= 2) {
            const p: [*]const u8 = @ptrFromInt(ptr);
            if (p[0] == ':' and p[1] == ':') return .{ .ptr = ptr + 2, .len = len - 2 };
        }
        return .{ .ptr = ptr, .len = len };
    }

    /// Called from the global-variable write path with the *stripped*
    /// name being written.  Marks every active vwait frame whose
    /// target matches.  Cheap fast-path: if the stack is empty (the
    /// common case — no vwait active) this is a single-byte read.
    pub fn note_var_write(self: *Stack, name_ptr: u32, name_len: u32) void {
        if (self.len == 0) return;
        const wn = strip_global(name_ptr, name_len);
        var i: u32 = 0;
        while (i < self.len) : (i += 1) {
            const f = &self.frames[i];
            const fn_name = strip_global(f.var_name_ptr, f.var_name_len);
            if (fn_name.len != wn.len) continue;
            const a: [*]const u8 = @ptrFromInt(fn_name.ptr);
            const b: [*]const u8 = @ptrFromInt(wn.ptr);
            var k: u32 = 0;
            var match = true;
            while (k < fn_name.len) : (k += 1) {
                if (a[k] != b[k]) { match = false; break; }
            }
            if (match) f.done = true;
        }
    }

    pub fn current_done(self: *Stack) bool {
        if (self.len == 0) return false;
        return self.frames[self.len - 1].done;
    }
};
