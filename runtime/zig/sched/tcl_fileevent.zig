// Fileevent registration table.
//
// Stage-1 scope: registration round-trip only.  Scripts can ask the
// scheduler "do I have a readable handler installed for chan X?"
// and the scheduler answers consistently.  Actual fd readiness via
// WASI ``poll_oneoff`` lands in Stage 3 (see
// ``docs/design/runtime/event-loop.md``); until then ``update`` /
// ``vwait`` ignore the fd-event table entirely.
//
// Registrations are keyed by channel name (as a TclObj), since our
// channel layer surfaces names like ``stdin`` / ``stdout`` /
// ``filechanN`` rather than raw fds — the latter only become useful
// once Stage 3 needs them for ``poll_oneoff``.  Each channel can
// carry one read handler and one write handler (matching real Tcl).
// Setting a handler to the empty string deregisters it, also
// matching real Tcl.

const obj = @import("../valtypes/tcl_obj.zig");

pub const Entry = struct {
    chan_name_ptr: u32,
    chan_name_len: u32,
    read_script: i32,
    write_script: i32,
    next: u32, // address of next Entry, 0 == nil
};

pub const Table = struct {
    head: u32 = 0,

    fn node(addr: u32) *Entry {
        return @ptrFromInt(addr);
    }

    fn name_eq(e: *const Entry, name_ptr: u32, name_len: u32) bool {
        if (e.chan_name_len != name_len) return false;
        const a: [*]const u8 = @ptrFromInt(e.chan_name_ptr);
        const b: [*]const u8 = @ptrFromInt(name_ptr);
        var i: u32 = 0;
        while (i < name_len) : (i += 1) {
            if (a[i] != b[i]) return false;
        }
        return true;
    }

    fn find_or_create(self: *Table, name_ptr: u32, name_len: u32) *Entry {
        var cur: u32 = self.head;
        while (cur != 0) {
            const e = node(cur);
            if (name_eq(e, name_ptr, name_len)) return e;
            cur = e.next;
        }
        // Create a new entry with a heap-copy of the channel name —
        // the caller's TclObj string buffer might be released
        // before we next consult the table.
        const buf = obj.alloc(name_len);
        if (name_len > 0) obj.memcpy(buf, name_ptr, name_len);
        const addr = obj.alloc(@sizeOf(Entry));
        const e = node(addr);
        e.chan_name_ptr = buf;
        e.chan_name_len = name_len;
        e.read_script = 0;
        e.write_script = 0;
        e.next = self.head;
        self.head = addr;
        return e;
    }

    /// Look up an existing entry without creating one.  Used by the
    /// clear-on-empty-script path so probing/clearing arbitrary
    /// channel names doesn't leak permanent table entries (Copilot
    /// review on PR #284).
    fn find(self: *const Table, name_ptr: u32, name_len: u32) ?*Entry {
        var cur: u32 = self.head;
        while (cur != 0) {
            const e = node(cur);
            if (name_eq(e, name_ptr, name_len)) return e;
            cur = e.next;
        }
        return null;
    }

    pub fn set_readable(self: *Table, name_ptr: u32, name_len: u32, script_obj: i32) void {
        if (script_obj == 0) {
            // Clearing: only update an existing entry; don't allocate
            // a tombstone for a never-seen channel.
            if (self.find(name_ptr, name_len)) |e| e.read_script = 0;
            return;
        }
        const e = self.find_or_create(name_ptr, name_len);
        e.read_script = script_obj;
    }

    pub fn set_writable(self: *Table, name_ptr: u32, name_len: u32, script_obj: i32) void {
        if (script_obj == 0) {
            if (self.find(name_ptr, name_len)) |e| e.write_script = 0;
            return;
        }
        const e = self.find_or_create(name_ptr, name_len);
        e.write_script = script_obj;
    }

    pub fn get_readable(self: *const Table, name_ptr: u32, name_len: u32) i32 {
        var cur: u32 = self.head;
        while (cur != 0) {
            const e = node(cur);
            if (name_eq(e, name_ptr, name_len)) return e.read_script;
            cur = e.next;
        }
        return 0;
    }

    pub fn get_writable(self: *const Table, name_ptr: u32, name_len: u32) i32 {
        var cur: u32 = self.head;
        while (cur != 0) {
            const e = node(cur);
            if (name_eq(e, name_ptr, name_len)) return e.write_script;
            cur = e.next;
        }
        return 0;
    }
};
