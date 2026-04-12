// Global variable table — open-addressing hash table with linear probing.

const obj = @import("tcl_obj.zig");
const alloc = obj.alloc;
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;
const memcpy = obj.memcpy;
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_int = obj.obj_new_int;
const obj_get_int = obj.obj_get_int;

const HTAB_BUCKET_SIZE: u32 = 16;
const HTAB_INITIAL_CAP: u32 = 16;

var htab_buf: u32 = 0;
var htab_cap: u32 = 0;
var htab_count: u32 = 0;

fn htab_init() void {
    if (htab_buf != 0) return;
    htab_cap = HTAB_INITIAL_CAP;
    htab_buf = alloc(htab_cap * HTAB_BUCKET_SIZE);
    var i: u32 = 0;
    while (i < htab_cap) : (i += 1) {
        write_i32(htab_buf + i * HTAB_BUCKET_SIZE, 0);
    }
}

pub fn fnv1a(ptr: u32, len: u32) u32 {
    // Empty string (common: ``$arr()`` with an empty key, or an
    // empty TclObj with ptr=0 from obj_ensure_string).  Skip the
    // ``@ptrFromInt`` — Zig's safe-mode checks panic on a null
    // pointer conversion, which is especially noisy when the
    // payload is a zero-length slice that wouldn't be dereferenced
    // anyway.
    if (len == 0) return 2166136261;
    var h: u32 = 2166136261;
    const src: [*]const u8 = @ptrFromInt(ptr);
    for (0..len) |i| {
        h ^= @as(u32, src[i]);
        h *%= 16777619;
    }
    return h;
}

fn htab_find(name_ptr: u32, name_len: u32, hash: u32) ?u32 {
    if (htab_buf == 0) return null;
    const mask = htab_cap - 1;
    var idx = hash & mask;
    var probes: u32 = 0;
    while (probes < htab_cap) : (probes += 1) {
        const base = htab_buf + idx * HTAB_BUCKET_SIZE;
        const ep: u32 = @bitCast(read_i32(base));
        if (ep == 0) return null;
        const el: u32 = @bitCast(read_i32(base + 4));
        const eh: u32 = @bitCast(read_i32(base + 8));
        if (eh == hash and el == name_len) {
            const sp: [*]const u8 = @ptrFromInt(ep);
            const np: [*]const u8 = @ptrFromInt(name_ptr);
            var match = true;
            for (0..el) |k| {
                if (sp[k] != np[k]) {
                    match = false;
                    break;
                }
            }
            if (match) return base;
        }
        idx = (idx + 1) & mask;
    }
    return null;
}

fn htab_insert(name_ptr: u32, name_len: u32, hash: u32, value: i32) void {
    const mask = htab_cap - 1;
    var idx = hash & mask;
    while (true) {
        const base = htab_buf + idx * HTAB_BUCKET_SIZE;
        const ep: u32 = @bitCast(read_i32(base));
        if (ep == 0) {
            const nbuf = alloc(name_len);
            memcpy(nbuf, name_ptr, name_len);
            write_i32(base, @bitCast(nbuf));
            write_i32(base + 4, @bitCast(name_len));
            write_i32(base + 8, @bitCast(hash));
            write_i32(base + 12, value);
            htab_count += 1;
            return;
        }
        idx = (idx + 1) & mask;
    }
}

fn htab_grow() void {
    const old_buf = htab_buf;
    const old_cap = htab_cap;
    htab_cap = old_cap * 2;
    htab_buf = alloc(htab_cap * HTAB_BUCKET_SIZE);
    htab_count = 0;
    var i: u32 = 0;
    while (i < htab_cap) : (i += 1) {
        write_i32(htab_buf + i * HTAB_BUCKET_SIZE, 0);
    }
    i = 0;
    while (i < old_cap) : (i += 1) {
        const base = old_buf + i * HTAB_BUCKET_SIZE;
        const ep: u32 = @bitCast(read_i32(base));
        if (ep != 0) {
            const el: u32 = @bitCast(read_i32(base + 4));
            const eh: u32 = @bitCast(read_i32(base + 8));
            const ev: i32 = read_i32(base + 12);
            const mask = htab_cap - 1;
            var idx = eh & mask;
            while (true) {
                const nb = htab_buf + idx * HTAB_BUCKET_SIZE;
                if (@as(u32, @bitCast(read_i32(nb))) == 0) {
                    write_i32(nb, @bitCast(ep));
                    write_i32(nb + 4, @bitCast(el));
                    write_i32(nb + 8, @bitCast(eh));
                    write_i32(nb + 12, ev);
                    htab_count += 1;
                    break;
                }
                idx = (idx + 1) & mask;
            }
        }
    }
}

pub export fn global_set(name: i32, value: i32) i32 {
    const sn = obj_ensure_string(name);
    htab_init();
    const hash = fnv1a(sn.ptr, sn.len);
    if (htab_find(sn.ptr, sn.len, hash)) |base| {
        write_i32(base + 12, value);
        return value;
    }
    if (htab_count * 4 >= htab_cap * 3) {
        htab_grow();
    }
    htab_insert(sn.ptr, sn.len, hash, value);
    return value;
}

pub export fn global_get(name: i32) i32 {
    const sn = obj_ensure_string(name);
    if (htab_buf == 0) return 0;
    const hash = fnv1a(sn.ptr, sn.len);
    if (htab_find(sn.ptr, sn.len, hash)) |base| {
        return read_i32(base + 12);
    }
    return 0;
}

pub export fn global_exists(name: i32) i32 {
    const sn = obj_ensure_string(name);
    if (htab_buf == 0) return obj_new_int(0);
    const hash = fnv1a(sn.ptr, sn.len);
    if (htab_find(sn.ptr, sn.len, hash) != null) {
        return obj_new_int(1);
    }
    return obj_new_int(0);
}

pub export fn tcl_incr(o: i32, amount: i32) i32 {
    const val = obj_get_int(o);
    const amt = obj_get_int(amount);
    return obj_new_int(val + amt);
}
