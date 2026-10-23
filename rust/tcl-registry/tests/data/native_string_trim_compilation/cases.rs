const TRIM_INPUTS: [&[u8]; 6] = [b" x ", b"x", b"", b"\0x\0", b"\xffx\xff", b"--x--"];
fn trim_header(
    snapshot: &tcl_syntax::native_object::NativeObjectSnapshot,
    references: usize,
) -> String {
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    let name = match snapshot.cache {
        Cache::None => "none",
        Cache::String { .. } | Cache::JimString { .. } => "string",
        Cache::ByteArray { .. } => "bytearray",
        ref other => panic!("unexpected trim primary {other:?}"),
    };
    format!(
        "{name},{},{}",
        usize::from(snapshot.resident.is_some()),
        references
    )
}
fn trim_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut output, byte| {
        write!(&mut output, "{byte:02x}").expect("writing to String");
        output
    })
}
