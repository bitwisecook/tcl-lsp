const INFO_COMMAND_NAMES: [&[u8]; 4] =
    [b"raw\xff", b"raw\xc3\xbf", b"N::raw\xff", b"N::raw\xc3\xbf"];
const INFO_COMMAND_PATTERNS: [&[u8]; 10] = [
    b"raw*",
    b"raw?",
    b"raw[\xff]",
    b"raw\xff",
    b"raw\xc3\xbf",
    b"::N::raw*",
    b"::N::raw?",
    b"::N::raw[\xff]",
    b"::raw?",
    b"raw\0::N::*",
];
fn info_command_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut output, byte| {
        write!(&mut output, "{byte:02x}").expect("writing to String");
        output
    })
}
