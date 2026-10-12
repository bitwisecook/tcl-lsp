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

#[test]
fn original_filename_external_zero_has_separate_backend_and_report_extents() {
    // naming.source.original-counted-filename-filesystem-extent
    // docs/design/analysis/name-resolution-proofs/source-original-counted-filename-filesystem-extent.md
    let mut stages = Stages {
        events: Vec::new(),
        read: Err(HostError::NotFound),
        withdraw_after_read: false,
        original_failure: false,
    };
    for suffix in [b"\0TAIL".as_slice(), b"\xc0\x80TAIL".as_slice()] {
        let original = [b"/source.tcl".as_slice(), suffix].concat();
        let selected = prepare_source_filename(
            &mut stages,
            NativeSourceFileProtocol::CSystem(TclVersion::V8_4),
            SystemEncoding::Utf8,
            &original,
        )
        .unwrap();
        assert_eq!(selected.backend, b"/source.tcl");
        assert_eq!(selected.label, original);
        assert_eq!(
            selected.reported,
            tcl_core_types::c_string_extent(&original)
        );
        for version in [
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let error = prepare_source_filename(
                &mut stages,
                NativeSourceFileProtocol::CSystem(version),
                SystemEncoding::Utf8,
                &original,
            )
            .err()
            .unwrap()
            .into_byte_details();
            assert_eq!(
                error.message,
                [
                    b"couldn't read file \"".as_slice(),
                    tcl_core_types::c_string_extent(&original),
                    b"\": invalid argument"
                ]
                .concat()
            );
            assert_eq!(
                error.error_code,
                crate::CmdErrorCodeUpdate::Set(b"POSIX EINVAL {invalid argument}".to_vec())
            );
        }
    }
    let original = b"/source.tcl\xc0\x80TAIL";
    let jim = prepare_source_filename(
        &mut stages,
        NativeSourceFileProtocol::Jim,
        SystemEncoding::Utf8,
        original,
    )
    .unwrap();
    assert_eq!(jim.backend, original);
    let raw_zero = prepare_source_filename(
        &mut stages,
        NativeSourceFileProtocol::Jim,
        SystemEncoding::Utf8,
        b"/source.tcl\0TAIL",
    )
    .unwrap();
    assert_eq!(raw_zero.backend, b"/source.tcl");
    assert!(stages.events.is_empty());
}

#[test]
fn original_filename_other_codec_requires_actual_filesystem_conversion() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut stages = Stages {
        events: Vec::new(),
        read: Ok(b"return YES".to_vec()),
        withdraw_after_read: false,
        original_failure: false,
    };
    let error = prepare_source_filename(
        &mut stages,
        NativeSourceFileProtocol::CSystem(TclVersion::V8_6),
        SystemEncoding::Ascii,
        b"/source.tcl",
    )
    .err()
    .unwrap();
    assert!(error.native_execution_refusal().is_some());
    assert!(stages.events.is_empty());
}

struct ChangingEncoding {
    stages: Stages,
    current: Cell<SystemEncoding>,
}
impl OriginalSourceFileOps for ChangingEncoding {
    fn original_bytes(&mut self, index: usize) -> Result<Rc<[u8]>, CmdError> {
        let reached = self.stages.original_bytes(index);
        self.current.set(SystemEncoding::Utf8);
        reached
    }
    fn system_encoding(&self) -> SystemEncoding {
        self.current.get()
    }
    fn open_file(
        &mut self,
        path: &[u8],
    ) -> Result<Result<Box<dyn SourceFile>, HostError>, CmdError> {
        self.stages.open_file(path)
    }
    fn lookup_encoding(
        &mut self,
        protocol: NativeEncodingNameProtocol,
        name: &[u8],
    ) -> Result<ChannelEncoding, CmdError> {
        self.stages.lookup_encoding(protocol, name)
    }
    fn retain_failure(&mut self, error: &CmdError) {
        self.stages.retain_failure(error);
    }
    fn check(&mut self) -> Result<(), CmdError> {
        self.stages.check()
    }
}

#[test]
fn original_filename_uses_system_encoding_reached_after_string_callbacks() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    let mut ops = ChangingEncoding {
        stages: Stages {
            events: Vec::new(),
            read: Ok(b"return YES".to_vec()),
            withdraw_after_read: false,
            original_failure: false,
        },
        current: Cell::new(SystemEncoding::Ascii),
    };
    let prepared = prepare_source_file(
        &mut ops,
        NativeSourceFileProtocol::CSystem(TclVersion::V8_6),
        1,
        None,
    )
    .unwrap();
    assert_eq!(prepared.path, b"/source.tcl");
    assert_eq!(prepared.script, b"return YES");
    assert_eq!(ops.current.get(), SystemEncoding::Utf8);
    assert_eq!(ops.stages.events, ["path-string", "open"]);
}

#[test]
fn original_filename_iso88591_and_identity_select_actual_external_bytes() {
    // naming.source.original-counted-filename-filesystem-extent
    // docs/design/analysis/name-resolution-proofs/source-original-counted-filename-filesystem-extent.md
    let mut stages = Stages {
        events: Vec::new(),
        read: Ok(Vec::new()),
        withdraw_after_read: false,
        original_failure: false,
    };
    for version in TclVersion::ALL {
        for (original, expected) in [
            (b"/key-\xc3\xa9.tcl".as_slice(), b"/key-\xe9.tcl".as_slice()),
            (b"/key-\xff.tcl".as_slice(), b"/key-\xff.tcl".as_slice()),
        ] {
            let file = prepare_source_filename(
                &mut stages,
                NativeSourceFileProtocol::CSystem(version),
                SystemEncoding::Iso88591,
                original,
            )
            .unwrap();
            assert_eq!(file.backend, expected);
            assert_eq!(file.label, original);
        }
        let euro = prepare_source_filename(
            &mut stages,
            NativeSourceFileProtocol::CSystem(version),
            SystemEncoding::Iso88591,
            b"/key-\xe2\x82\xac.tcl",
        );
        if version < TclVersion::V9_0 {
            assert_eq!(euro.unwrap().backend, b"/key-?.tcl");
        } else {
            let error = euro.err().unwrap().into_byte_details();
            assert_eq!(error.message, b"couldn't read file \"/key-\xe2\x82\xac.tcl\": invalid or incomplete multibyte or wide character");
            assert_eq!(
                error.error_code,
                crate::CmdErrorCodeUpdate::Set(
                    b"POSIX EILSEQ {invalid or incomplete multibyte or wide character}".to_vec()
                )
            );
        }
    }
    for version in [TclVersion::V8_4, TclVersion::V8_5, TclVersion::V8_6] {
        let original = b"/key-\xc0\x80TAIL";
        let file = prepare_source_filename(
            &mut stages,
            NativeSourceFileProtocol::CSystem(version),
            SystemEncoding::Identity,
            original,
        )
        .unwrap();
        assert_eq!(file.backend, original);
    }
    assert!(stages.events.is_empty());
}

#[test]
fn original_c9_identity_name_uses_actual_catalogue_instead_of_builtin_codec() {
    // naming.source.original-counted-filename-filesystem-extent
    // docs/design/analysis/name-resolution-proofs/source-original-counted-filename-filesystem-extent.md
    let queries = Cell::new(0);
    let error = resolve_original_encoding_name(
        NativeEncodingNameProtocol::for_c(TclVersion::V9_0),
        NativeEncodingNamePurpose::System(SystemEncoding::Utf8),
        b"identity",
        |_| {
            queries.set(queries.get() + 1);
            Ok(None)
        },
    )
    .unwrap_err();
    assert_eq!(
        error.into_byte_details().message,
        b"unknown encoding \"identity\""
    );
    assert_eq!(queries.get(), 1);
    let supplied = resolve_original_encoding_name(
        NativeEncodingNameProtocol::for_c(TclVersion::V9_0),
        NativeEncodingNamePurpose::System(SystemEncoding::Utf8),
        b"identity",
        |_| Ok(Some(SystemEncoding::Identity)),
    )
    .unwrap();
    assert_eq!(supplied, ChannelEncoding::Identity);
}

#[test]
fn original_filename_utf8_surrogates_keep_selected_conversion_and_error_owner() {
    // naming.source.original-counted-filename-filesystem-extent
    // docs/design/analysis/name-resolution-proofs/source-original-counted-filename-filesystem-extent.md
    let mut stages = Stages {
        events: Vec::new(),
        read: Ok(Vec::new()),
        withdraw_after_read: false,
        original_failure: false,
    };
    for version in TclVersion::ALL {
        for original in [
            b"/key-\xed\xa0\x80.tcl".as_slice(),
            b"/key-\xed\xb0\x80.tcl".as_slice(),
            b"/key-\xed\xa0\xbd\xed\xb8\x80.tcl".as_slice(),
        ] {
            let reached = prepare_source_filename(
                &mut stages,
                NativeSourceFileProtocol::CSystem(version),
                SystemEncoding::Utf8,
                original,
            );
            if version >= TclVersion::V9_0 {
                let error = reached.err().unwrap();
                assert!(error.native_execution_refusal().is_none());
                let details = error.into_byte_details();
                assert_eq!(
                    details.message,
                    [
                        b"couldn't read file \"".as_slice(),
                        original,
                        b"\": invalid or incomplete multibyte or wide character"
                    ]
                    .concat()
                );
                assert_eq!(
                    details.error_code,
                    crate::CmdErrorCodeUpdate::Set(
                        b"POSIX EILSEQ {invalid or incomplete multibyte or wide character}"
                            .to_vec()
                    )
                );
            } else {
                let file = reached.unwrap();
                let paired = original == b"/key-\xed\xa0\xbd\xed\xb8\x80.tcl";
                let expected = if paired && version == TclVersion::V8_6 {
                    b"/key-\xf0\x9f\x98\x80.tcl".as_slice()
                } else {
                    original
                };
                assert_eq!(file.backend, expected);
                assert_eq!(file.label, original);
            }
        }
    }
    assert!(stages.events.is_empty());
}
