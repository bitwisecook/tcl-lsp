// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use std::cell::Cell;

#[test]
fn original_byte_names_keep_c_extent_raw_diagnostics_and_actual_reset_default() {
    // naming.encoding.original-byte-name-lookup-and-source-order
    // docs/design/analysis/name-resolution-proofs/encoding-original-byte-name-lookup-and-source-order.md
    for version in TclVersion::ALL {
        let queries = Cell::new(0);
        let protocol = NativeEncodingNameProtocol::for_c(version);
        for (name, expected) in [
            (b"utf-8\0TAIL".as_slice(), SystemEncoding::Utf8),
            (b"\0TAIL".as_slice(), SystemEncoding::Identity),
        ] {
            let result = resolve_original_encoding_name(
                protocol,
                NativeEncodingNamePurpose::System(SystemEncoding::Identity),
                name,
                |_| {
                    queries.set(queries.get() + 1);
                    Ok(None)
                },
            )
            .unwrap();
            assert_eq!(result.system_encoding(), Some(expected));
        }
        assert_eq!(queries.get(), 0);
        for name in [
            b"\xff".as_slice(),
            b"UTF-8".as_slice(),
            b"utf-8\xc0\x80TAIL".as_slice(),
        ] {
            let error = resolve_original_encoding_name(
                protocol,
                NativeEncodingNamePurpose::System(SystemEncoding::Utf8),
                name,
                |_| Ok(None),
            )
            .unwrap_err();
            let details = error.into_byte_details();
            assert_eq!(
                details.message,
                [b"unknown encoding \"".as_slice(), name, b"\""].concat()
            );
            if version == TclVersion::V8_4 {
                assert_eq!(details.error_code, crate::CmdErrorCodeUpdate::Default);
            } else {
                assert_eq!(
                    details.error_code,
                    crate::CmdErrorCodeUpdate::Set(
                        [b"TCL LOOKUP ENCODING ".as_slice(), name].concat()
                    )
                );
            }
        }
    }
}

#[test]
fn original_channel_pseudo_encodings_stay_distinct_from_system_reset_and_catalogue_absence() {
    // naming.encoding.original-byte-name-lookup-and-source-order
    // docs/design/analysis/name-resolution-proofs/encoding-original-byte-name-lookup-and-source-order.md
    for version in TclVersion::ALL {
        for name in [b"".as_slice(), b"binary".as_slice()] {
            let result = resolve_original_encoding_name(
                NativeEncodingNameProtocol::for_c(version),
                NativeEncodingNamePurpose::Channel,
                name,
                |_| panic!("pseudo encoding does not query catalogue"),
            );
            if version < TclVersion::V9_0 {
                assert_eq!(result.unwrap(), ChannelEncoding::Tcl8Binary);
            } else {
                let error = result.unwrap_err().into_byte_details();
                assert_eq!(error.error_code, crate::CmdErrorCodeUpdate::Default);
                assert!(error.message.ends_with(b"-encoding iso8859-1\""));
            }
        }
    }
    let error = resolve_original_encoding_name(
        NativeEncodingNameProtocol::for_c(TclVersion::V9_0),
        NativeEncodingNamePurpose::System(SystemEncoding::Ascii),
        b"external-codec",
        |_| Err(ValueError::CommandProtocolUnavailable("catalogue unavailable").into()),
    )
    .unwrap_err();
    assert!(error.native_execution_refusal().is_some());
}

struct TestSourceFile(Vec<u8>);
impl SourceFile for TestSourceFile {
    fn read_all(&mut self) -> Result<Vec<u8>, HostError> {
        Ok(self.0.clone())
    }
}

struct Stages {
    events: Vec<&'static str>,
    read: Result<Vec<u8>, HostError>,
    withdraw_after_read: bool,
    original_failure: bool,
}
impl OriginalSourceFileOps for Stages {
    fn original_bytes(&mut self, index: usize) -> Result<Rc<[u8]>, CmdError> {
        self.events.push(if index == 0 {
            "encoding-string"
        } else {
            "path-string"
        });
        if self.original_failure && index == 0 {
            return Err(ValueError::CommandProtocolUnavailable("original String withdrawn").into());
        }
        Ok(Rc::from(if index == 0 {
            b"missing-codec".as_slice()
        } else {
            b"/source.tcl".as_slice()
        }))
    }
    fn system_encoding(&self) -> SystemEncoding {
        SystemEncoding::Utf8
    }
    fn open_file(
        &mut self,
        _path: &[u8],
    ) -> Result<Result<Box<dyn SourceFile>, HostError>, CmdError> {
        self.events.push("open");
        Ok(self
            .read
            .clone()
            .map(|bytes| Box::new(TestSourceFile(bytes)) as Box<dyn SourceFile>))
    }
    fn lookup_encoding(
        &mut self,
        protocol: NativeEncodingNameProtocol,
        name: &[u8],
    ) -> Result<ChannelEncoding, CmdError> {
        self.events.push("lookup");
        resolve_original_encoding_name(protocol, NativeEncodingNamePurpose::Channel, name, |_| {
            Ok(None)
        })
    }
    fn retain_failure(&mut self, _error: &CmdError) {}
    fn check(&mut self) -> Result<(), CmdError> {
        if self.withdraw_after_read && self.events.contains(&"open") {
            Err(ValueError::CommandProtocolUnavailable("original operation withdrawn").into())
        } else {
            Ok(())
        }
    }
}

#[test]
fn original_source_stage_failure_stops_before_later_lookup_or_backend() {
    // naming.encoding.original-byte-name-lookup-and-source-order
    // docs/design/analysis/name-resolution-proofs/encoding-original-byte-name-lookup-and-source-order.md
    let mut stages = Stages {
        events: Vec::new(),
        read: Err(HostError::NotFound),
        withdraw_after_read: false,
        original_failure: false,
    };
    let error = prepare_source_file(
        &mut stages,
        NativeSourceFileProtocol::CUtf8(TclVersion::V9_0),
        1,
        Some(0),
    )
    .err()
    .unwrap();
    assert_eq!(stages.events, ["encoding-string", "path-string", "open"]);
    assert_eq!(
        error.error_code_bytes(),
        Some(b"POSIX ENOENT {no such file or directory}".as_slice())
    );
    stages.events.clear();
    stages.original_failure = true;
    let error = prepare_source_file(
        &mut stages,
        NativeSourceFileProtocol::CUtf8(TclVersion::V9_0),
        1,
        Some(0),
    )
    .err()
    .unwrap();
    assert!(error.native_execution_refusal().is_some());
    assert_eq!(stages.events, ["encoding-string"]);
    stages.events.clear();
    stages.original_failure = false;
    stages.withdraw_after_read = true;
    stages.read = Ok(b"return YES".to_vec());
    let error = prepare_source_file(
        &mut stages,
        NativeSourceFileProtocol::CUtf8(TclVersion::V9_0),
        1,
        Some(0),
    )
    .err()
    .unwrap();
    assert!(error.native_execution_refusal().is_some());
    assert_eq!(stages.events, ["encoding-string", "path-string", "open"]);
}
