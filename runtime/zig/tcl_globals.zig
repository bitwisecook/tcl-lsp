// Global variable table — open-addressing hash table built on the
// shared ``hash_table.zig`` primitive.
//
// Bucket layout (16 bytes): 12-byte header (name_ptr | name_len |
// hash) + 4-byte value (i32 TclObj handle).

const obj = @import("tcl_obj.zig");
const read_i32 = obj.read_i32;
const write_i32 = obj.write_i32;
const obj_ensure_string = obj.obj_ensure_string;
const obj_new_int = obj.obj_new_int;
const obj_get_int = obj.obj_get_int;

const ht = @import("hash_table.zig");
const fnv1a = ht.fnv1a;

const HTAB_BUCKET_SIZE: u32 = 16;
const HTAB_INITIAL_CAP: u32 = 16;
const OFF_VALUE: u32 = 12;

const GlobalTable = ht.Table(HTAB_BUCKET_SIZE);
var htab: GlobalTable = .{};

pub export fn global_set(name: i32, value: i32) i32 {
    const sn = obj_ensure_string(name);
    htab.init(HTAB_INITIAL_CAP);
    const hash = fnv1a(sn.ptr, sn.len);
    if (htab.find(sn.ptr, sn.len, hash)) |base| {
        write_i32(base + OFF_VALUE, value);
        return value;
    }
    if (htab.needs_grow()) htab.grow();
    const base = htab.insert_header(sn.ptr, sn.len, hash);
    write_i32(base + OFF_VALUE, value);
    return value;
}

pub export fn global_get(name: i32) i32 {
    const sn = obj_ensure_string(name);
    if (htab.buf == 0) return 0;
    const hash = fnv1a(sn.ptr, sn.len);
    if (htab.find(sn.ptr, sn.len, hash)) |base| {
        return read_i32(base + OFF_VALUE);
    }
    return 0;
}

pub export fn global_exists(name: i32) i32 {
    const sn = obj_ensure_string(name);
    if (htab.buf == 0) return obj_new_int(0);
    const hash = fnv1a(sn.ptr, sn.len);
    if (htab.find(sn.ptr, sn.len, hash) != null) {
        return obj_new_int(1);
    }
    return obj_new_int(0);
}

pub export fn tcl_incr(o: i32, amount: i32) i32 {
    const val = obj_get_int(o);
    const amt = obj_get_int(amount);
    return obj_new_int(val + amt);
}
