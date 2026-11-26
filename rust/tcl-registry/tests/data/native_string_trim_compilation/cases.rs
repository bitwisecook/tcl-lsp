const TRIM_INPUTS: [&[u8]; 6] = [b" x ", b"x", b"", b"\0x\0", b"\xffx\xff", b"--x--"];
fn trim_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut output, byte| {
        write!(&mut output, "{byte:02x}").expect("writing to String");
        output
    })
}
