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
