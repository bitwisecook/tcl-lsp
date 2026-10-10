// Fixed Rust wasm32-wasip1 libc observation. No Tcl or engine getters.
// C prototypes and errno labels follow the pinned libc0.2.190 WASI binding.
// Sizes/limits below are queried from the actual compiled Rust C-FFI aliases,
// not C SDK macros. The reached library and its link closure are independent
// from the SDK C probe and from any Runtime cdylib or Tcl getter recipe.
use std::ffi::{
    c_char, c_double, c_int, c_long, c_longlong, c_uint, c_ulong, c_ulonglong, c_void, CStr,
};
use std::mem::size_of;

#[cfg(not(all(target_arch = "wasm32", target_os = "wasi", target_env = "p1")))]
compile_error!("this original probe requires wasm32-wasip1");

#[link(name = "c")]
unsafe extern "C" {
    fn __errno_location() -> *mut c_int;
    fn strtol(input: *const c_char, end: *mut *mut c_char, base: c_int) -> c_long;
    fn strtoul(input: *const c_char, end: *mut *mut c_char, base: c_int) -> c_ulong;
    fn strtoull(input: *const c_char, end: *mut *mut c_char, base: c_int) -> c_ulonglong;
    fn strtod(input: *const c_char, end: *mut *mut c_char) -> c_double;
    fn printf(format: *const c_char, ...) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

const EDOM: c_int = 18;
const ERANGE: c_int = 68;
const EINVAL: c_int = 28;

struct InputCase {
    name: &'static CStr,
    bytes: &'static [u8],
    length: usize,
}
const CASES: &[InputCase] = &[
    InputCase {
        name: c"empty",
        bytes: &[0],
        length: 0,
    },
    InputCase {
        name: c"bad",
        bytes: &[98, 97, 100, 0],
        length: 3,
    },
    InputCase {
        name: c"hex16",
        bytes: &[48, 120, 49, 48, 0],
        length: 4,
    },
    InputCase {
        name: c"legacy_octal08",
        bytes: &[48, 56, 0],
        length: 2,
    },
    InputCase {
        name: c"space_sign_tail",
        bytes: &[32, 9, 45, 49, 55, 116, 97, 105, 108, 0],
        length: 9,
    },
    InputCase {
        name: c"positive_sign",
        bytes: &[43, 49, 55, 0],
        length: 3,
    },
    InputCase {
        name: c"negative_zero",
        bytes: &[45, 48, 0],
        length: 2,
    },
    InputCase {
        name: c"negative_one",
        bytes: &[45, 49, 0],
        length: 2,
    },
    InputCase {
        name: c"signed32_max",
        bytes: &[50, 49, 52, 55, 52, 56, 51, 54, 52, 55, 0],
        length: 10,
    },
    InputCase {
        name: c"signed32_overflow",
        bytes: &[50, 49, 52, 55, 52, 56, 51, 54, 52, 56, 0],
        length: 10,
    },
    InputCase {
        name: c"signed32_min",
        bytes: &[45, 50, 49, 52, 55, 52, 56, 51, 54, 52, 56, 0],
        length: 11,
    },
    InputCase {
        name: c"signed32_underflow",
        bytes: &[45, 50, 49, 52, 55, 52, 56, 51, 54, 52, 57, 0],
        length: 11,
    },
    InputCase {
        name: c"unsigned64_max",
        bytes: &[
            49, 56, 52, 52, 54, 55, 52, 52, 48, 55, 51, 55, 48, 57, 53, 53, 49, 54, 49, 53, 0,
        ],
        length: 20,
    },
    InputCase {
        name: c"unsigned64_overflow",
        bytes: &[
            49, 56, 52, 52, 54, 55, 52, 52, 48, 55, 51, 55, 48, 57, 53, 53, 49, 54, 49, 54, 0,
        ],
        length: 20,
    },
    InputCase {
        name: c"nan",
        bytes: &[78, 97, 78, 0],
        length: 3,
    },
    InputCase {
        name: c"positive_infinity",
        bytes: &[73, 110, 102, 0],
        length: 3,
    },
    InputCase {
        name: c"negative_infinity",
        bytes: &[45, 73, 110, 102, 0],
        length: 4,
    },
    InputCase {
        name: c"double_overflow",
        bytes: &[49, 101, 53, 48, 48, 48, 0],
        length: 6,
    },
    InputCase {
        name: c"double_underflow",
        bytes: &[49, 101, 45, 53, 48, 48, 48, 0],
        length: 7,
    },
    InputCase {
        name: c"counted_one_nul_x",
        bytes: &[49, 0, 88, 0],
        length: 3,
    },
    InputCase {
        name: c"counted_initial_nul_one",
        bytes: &[0, 49, 0],
        length: 2,
    },
];

fn errno_cell() -> *mut c_int {
    // SAFETY: the selected guest libc exposes this guest thread's errno cell.
    unsafe { __errno_location() }
}
fn errno_get() -> c_int {
    // SAFETY: the pointer is consumed on this same guest thread immediately.
    unsafe { *errno_cell() }
}
fn errno_set(value: c_int) {
    // SAFETY: only this guest thread's current libc errno cell is changed.
    unsafe { *errno_cell() = value }
}
fn prefix_length(input: &InputCase) -> usize {
    input.bytes[..input.length]
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(input.length)
}
fn hex(bytes: &[u8]) {
    for byte in bytes {
        // SAFETY: fixed NUL-terminated format; default-promoted argument matches.
        unsafe {
            printf(c"%02x".as_ptr(), c_uint::from(*byte));
        }
    }
}
fn observe(index: usize, seed_kind: c_int, seed: c_int, worker: c_int) {
    let input = &CASES[index];
    let start = input.bytes.as_ptr().cast::<c_char>();
    let mut end = std::ptr::null_mut();
    errno_set(seed);
    let before = errno_get();
    if worker == 4 {
        errno_set(0);
    }
    let entry_errno = errno_get();
    let mut signed_value: c_long = 0;
    let mut unsigned_value: c_ulong = 0;
    let mut wide_value: c_ulonglong = 0;
    let mut double_value: c_double = 0.0;
    // SAFETY: original bytes include a final NUL, remain alive, and end points
    // to writable local storage. All original numeric calls select radix0.
    unsafe {
        match worker {
            0 => signed_value = strtol(start, &raw mut end, 0),
            1 => unsigned_value = strtoul(start, &raw mut end, 0),
            2 => wide_value = strtoull(start, &raw mut end, 0),
            3 | 4 => double_value = strtod(start, &raw mut end),
            _ => unreachable!("fixed worker vector"),
        }
    }
    let reached_errno = errno_get(); // Before formatting or float classification.
    assert!(
        !end.is_null(),
        "libc did not supply the original end pointer"
    );
    // SAFETY: the original libc end pointer lies in this same static allocation.
    let consumed = unsafe { end.offset_from(start) };
    // SAFETY: formats and each varargs type correspond to the actual FFI aliases.
    unsafe {
        printf(c"ROW\tcase=%zu\tworker=%d\tseed_kind=%d\tseed=%d\tbefore=%d\tentry_errno=%d\treached_errno=%d\tend=%td\tinput_len=%zu\tprefix_len=%zu".as_ptr(),
            index, worker, seed_kind, seed, before, entry_errno, reached_errno,
            consumed, input.length, prefix_length(input));
        match worker {
            0 => {
                printf(c"\tvalue=%ld".as_ptr(), signed_value);
            }
            1 => {
                printf(c"\tvalue=%lu".as_ptr(), unsigned_value);
            }
            2 => {
                printf(c"\tvalue=%llu".as_ptr(), wide_value);
            }
            3 | 4 => {
                printf(c"\tvalue=%a\tdouble_bytes=".as_ptr(), double_value);
                hex(&double_value.to_ne_bytes());
                printf(
                    c"\tis_nan=%d\tis_infinite=%d\tsignbit=%d".as_ptr(),
                    c_int::from(double_value.is_nan()),
                    c_int::from(double_value.is_infinite()),
                    c_int::from(double_value.is_sign_negative()),
                );
            }
            _ => unreachable!("fixed worker vector"),
        }
        printf(c"\n".as_ptr());
    }
}
fn adapter_boundary(seed_kind: c_int, seed: c_int, offset: usize) {
    let bytes = b"1\0X\0";
    let counted_length = 3usize;
    let prefix = 1usize;
    let start = bytes.as_ptr().cast::<c_char>();
    errno_set(seed);
    let before = errno_get();
    if offset > prefix {
        let reached_errno = errno_get();
        // SAFETY: fixed format and all matching varargs; no libc numeric call.
        unsafe {
            printf(c"ADAPTER\tseed_kind=%d\tseed=%d\toffset=%zu\tcall=rejected\tbefore=%d\treached_errno=%d\tinput_len=%zu\tprefix_len=%zu\n".as_ptr(),
                seed_kind, seed, offset, before, reached_errno, counted_length, prefix);
        }
        return;
    }
    let mut end = std::ptr::null_mut();
    // SAFETY: admitted offset does not exceed the first NUL in the live input.
    let value = unsafe { strtoull(start.add(offset), &raw mut end, 0) };
    let reached_errno = errno_get();
    assert!(
        !end.is_null(),
        "libc did not supply the admitted end pointer"
    );
    // SAFETY: the returned end pointer belongs to the same original allocation.
    let consumed = unsafe { end.offset_from(start) };
    // SAFETY: fixed format with matching FFI aliases and pointer-width integers.
    unsafe {
        printf(c"ADAPTER\tseed_kind=%d\tseed=%d\toffset=%zu\tcall=executed\tbefore=%d\treached_errno=%d\tvalue=%llu\tend=%td\tinput_len=%zu\tprefix_len=%zu\n".as_ptr(),
            seed_kind, seed, offset, before, reached_errno, value, consumed, counted_length, prefix);
    }
}
fn main() {
    // SAFETY: every format matches the actual target's queried FFI value types.
    unsafe {
        printf(c"ABI\tsource=RustFFIaliases\tCHAR_BIT=%u\tchar=%zu\tint=%zu\tlong=%zu\tlong_long=%zu\tdouble=%zu\tpointer=%zu\tCHAR_MIN=%d\tCHAR_MAX=%d\tINT_MIN=%d\tINT_MAX=%d\tUINT_MAX=%u\tLONG_MIN=%ld\tLONG_MAX=%ld\tULONG_MAX=%lu\tLLONG_MIN=%lld\tLLONG_MAX=%lld\tULLONG_MAX=%llu\tEDOM_binding=%d\tERANGE_binding=%d\tEINVAL_binding=%d\tDBL_MANT_DIG=%u\tDBL_MIN_EXP=%d\tDBL_MAX_EXP=%d\tRUST_EDITION=2021\n".as_ptr(),
            u8::BITS, size_of::<c_char>(), size_of::<c_int>(), size_of::<c_long>(),
            size_of::<c_longlong>(), size_of::<c_double>(), size_of::<*const c_void>(),
            c_int::from(c_char::MIN), c_int::from(c_char::MAX), c_int::MIN, c_int::MAX,
            c_uint::MAX, c_long::MIN, c_long::MAX, c_ulong::MAX, c_longlong::MIN,
            c_longlong::MAX, c_ulonglong::MAX, EDOM, ERANGE, EINVAL,
            f64::MANTISSA_DIGITS, f64::MIN_EXP, f64::MAX_EXP);
    }
    let seeds = [0, EDOM, ERANGE];
    for (seed_kind, seed) in seeds.iter().copied().enumerate() {
        let first_cell = errno_cell();
        // SAFETY: both writes/read witnesses stay on the current guest thread.
        unsafe {
            *first_cell = seed;
        }
        let via_first = errno_get();
        errno_set(123);
        let via_second = unsafe { *first_cell };
        let same_cell = c_int::from(first_cell == errno_cell());
        errno_set(seed);
        let before_reset = errno_get();
        errno_set(0);
        let after_reset = errno_get();
        unsafe {
            printf(c"ERRNO_CELL\tseed_kind=%zu\tseed=%d\tvia_first=%d\tvia_second=%d\tsame_cell=%d\tbefore_reset=%d\tafter_reset=%d\n".as_ptr(),
                seed_kind, seed, via_first, via_second, same_cell, before_reset, after_reset);
        }
        assert!(via_first == seed && via_second == 123 && same_cell == 1 && after_reset == 0);
    }
    for (index, input) in CASES.iter().enumerate() {
        unsafe {
            printf(
                c"CASE\tcase=%zu\tname=%s\tinput_len=%zu\tprefix_len=%zu\tinput=".as_ptr(),
                index,
                input.name.as_ptr(),
                input.length,
                prefix_length(input),
            );
        }
        hex(&input.bytes[..input.length]);
        unsafe {
            printf(c"\n".as_ptr());
        }
        for (seed_kind, seed) in seeds.iter().copied().enumerate() {
            for worker in 0..5 {
                observe(index, seed_kind as c_int, seed, worker);
            }
        }
    }
    for (seed_kind, seed) in seeds.iter().copied().enumerate() {
        for offset in 0..=3 {
            adapter_boundary(seed_kind as c_int, seed, offset);
        }
    }
    unsafe {
        printf(
            c"SUMMARY\tcases=%zu\trows=%zu\terrno_cells=3\tadapter_rows=12\n".as_ptr(),
            CASES.len(),
            CASES.len() * 3 * 5,
        );
        if fflush(std::ptr::null_mut()) != 0 {
            std::process::exit(3);
        }
    }
}
