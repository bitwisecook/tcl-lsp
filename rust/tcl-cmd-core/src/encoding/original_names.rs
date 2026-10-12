// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original byte encoding names and source stages, independent of object issuers.

use crate::{
    CmdError,
    channel::{ChannelEncoding, decode_text},
};
use std::rc::Rc;
use tcl_dialect::TclVersion;
use tcl_platform::{HostError, SourceFile, SystemEncoding};
use tcl_syntax::{native_string::NativeStringProtocol, value::ValueError};

/// Actual C encoding catalogue presentation selected by an entered adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeEncodingNameProtocol {
    version: TclVersion,
}

impl NativeEncodingNameProtocol {
    /// Explicit independent C recipe; this supplies no original object authority.
    #[must_use]
    pub const fn for_c(version: TclVersion) -> Self {
        Self { version }
    }

    fn unknown(self, name: &[u8]) -> CmdError {
        let message = [b"unknown encoding \"".as_slice(), name, b"\""].concat();
        let error = if self.version == TclVersion::V8_4 {
            CmdError::new_bytes(message)
        } else {
            let mut code = b"TCL LOOKUP ENCODING".to_vec();
            tcl_syntax::list::append_list_element(&mut code, name, false);
            CmdError::with_error_code_bytes(message, code)
        };
        error.with_native_string_result(NativeStringProtocol::C(self.version))
    }
}

/// System reset is distinct from a channel's empty/binary pseudo-encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeEncodingNamePurpose {
    /// An empty name restores the separately captured actual Host default.
    System(SystemEncoding),
    /// Channel configuration, including the C8 pseudo-encoding spellings.
    Channel,
}

/// Resolve the original C-string extent without Unicode, case or whitespace repair.
/// The supplied catalogue callback is reached only for an unimplemented name.
///
/// # Errors
/// Returns actual catalogue failure or selected byte-exact missing-name diagnostics.
/// An absent catalogue must be represented by a typed unavailable callback result.
pub fn resolve_original_encoding_name(
    protocol: NativeEncodingNameProtocol,
    purpose: NativeEncodingNamePurpose,
    original: &[u8],
    catalogue: impl FnOnce(&[u8]) -> Result<Option<SystemEncoding>, CmdError>,
) -> Result<ChannelEncoding, CmdError> {
    // naming.encoding.original-byte-name-lookup-and-source-order
    // docs/design/analysis/name-resolution-proofs/encoding-original-byte-name-lookup-and-source-order.md
    let name = tcl_core_types::c_string_extent(original);
    if name.is_empty()
        && let NativeEncodingNamePurpose::System(default) = purpose
    {
        return Ok(ChannelEncoding::from_system(default));
    }
    if purpose == NativeEncodingNamePurpose::Channel && (name.is_empty() || name == b"binary") {
        return if protocol.version < TclVersion::V9_0 {
            Ok(ChannelEncoding::Tcl8Binary)
        } else {
            Err(CmdError::new_bytes([
                b"unknown encoding \"".as_slice(), name,
                b"\": No longer supported.\n\tplease use either \"-translation binary\" or \"-encoding iso8859-1\"",
            ].concat()).with_native_string_result(NativeStringProtocol::C(protocol.version)))
        };
    }
    // C9 has an anonymous binary encoding but no stock named identity codec.
    // A separately supplied catalogue may still establish an installed name.
    let known = if protocol.version >= TclVersion::V9_0 && name == b"identity" {
        None
    } else {
        implemented_encoding(name)
    };
    let encoding = match known {
        Some(encoding) => Some(encoding),
        None => catalogue(name)?,
    };
    encoding
        .map(ChannelEncoding::from_system)
        .ok_or_else(|| protocol.unknown(name))
}

pub(crate) fn implemented_encoding(name: &[u8]) -> Option<SystemEncoding> {
    match name {
        b"utf-8" => Some(SystemEncoding::Utf8),
        b"identity" => Some(SystemEncoding::Identity),
        b"iso8859-1" => Some(SystemEncoding::Iso88591),
        b"ascii" => Some(SystemEncoding::Ascii),
        b"unicode" => Some(SystemEncoding::Unicode),
        _ => None,
    }
}

/// Source default and result representation selected independently of file permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeSourceFileProtocol {
    /// C8 opens its source channel with the current system encoding.
    CSystem(TclVersion),
    /// C9 source explicitly selects UTF-8 when no encoding operand is supplied.
    CUtf8(TclVersion),
    /// Jim reads original bytes; it has no encoding option or catalogue command.
    Jim,
}

impl NativeSourceFileProtocol {
    /// Original C lookup recipe, absent on Jim's plain source path.
    #[must_use]
    pub const fn encoding(self) -> Option<NativeEncodingNameProtocol> {
        match self {
            Self::CSystem(version) | Self::CUtf8(version) => {
                Some(NativeEncodingNameProtocol::for_c(version))
            }
            Self::Jim => None,
        }
    }

    fn read_error(self, path: &[u8], error: HostError) -> CmdError {
        if error == HostError::Unsupported {
            return ValueError::CommandProtocolUnavailable("source filesystem byte path").into();
        }
        if matches!(error, HostError::Io(_)) {
            return ValueError::CommandProtocolUnavailable("source filesystem errno unavailable")
                .into();
        }
        let reason = if self == Self::Jim && error == HostError::NotFound {
            "No such file or directory".to_owned()
        } else {
            error.reason()
        };
        let message = [
            b"couldn't read file \"".as_slice(),
            path,
            b"\": ",
            reason.as_bytes(),
        ]
        .concat();
        if self == Self::Jim {
            let mut details = CmdError::new_bytes(message).into_byte_details();
            details.error_code = crate::CmdErrorCodeUpdate::Unchanged;
            return CmdError::from_byte_details(details);
        }
        let errno = match error {
            HostError::NotFound => b"ENOENT".as_slice(),
            HostError::Permission => b"EACCES".as_slice(),
            HostError::AlreadyExists => b"EEXIST".as_slice(),
            HostError::WouldBlock => b"EAGAIN".as_slice(),
            HostError::Io(_) | HostError::Unsupported => return CmdError::new_bytes(message),
        };
        let mut code = b"POSIX".to_vec();
        tcl_syntax::list::append_list_element(&mut code, errno, false);
        tcl_syntax::list::append_list_element(&mut code, reason.as_bytes(), false);
        CmdError::with_error_code_bytes(message, code)
    }
}

/// Thin runtime stages under one actual original Name/argv operation.
/// Each callback-bearing stage is followed by `check` before later work.
pub trait OriginalSourceFileOps {
    /// Reach one original counted String at its selected effective argv index.
    fn original_bytes(&mut self, index: usize) -> Result<Rc<[u8]>, CmdError>;
    /// Convert shared selected native units with this backend's actual codec.
    /// The reached system encoding is distinct from explicit source decoding.
    ///
    /// # Errors
    /// Returns typed unavailable when the actual filesystem lacks that converter.
    fn external_filename_bytes(
        &mut self,
        _units: &[u32],
        _encoding: SystemEncoding,
    ) -> Result<Vec<u8>, CmdError> {
        Err(ValueError::CommandProtocolUnavailable("source filesystem filename conversion").into())
    }
    /// Actual current system encoding at this reached source stage.
    fn system_encoding(&self) -> SystemEncoding;
    /// Reach the original file open without reading bytes; the handle is mandatory cleanup.
    fn open_file(
        &mut self,
        path: &[u8],
    ) -> Result<Result<Box<dyn SourceFile>, HostError>, CmdError>;
    /// Original byte catalogue lookup, after a successful backend boundary.
    fn lookup_encoding(
        &mut self,
        protocol: NativeEncodingNameProtocol,
        name: &[u8],
    ) -> Result<ChannelEncoding, CmdError>;
    /// Latch an actual typed stage refusal before mandatory handle cleanup.
    /// Genuine Guest diagnostics do not become Host refusal.
    fn retain_failure(&mut self, error: &CmdError);
    /// Same original operation/receiver check, preserving its full first cause.
    fn check(&mut self) -> Result<(), CmdError>;
}

/// Original path and decoded script; package/frame/evaluation ownership stays with the caller.
pub struct PreparedSourceFile {
    /// Original counted C source label, or the selected raw Jim C-string filename.
    pub path: Vec<u8>,
    /// Selected existing conversion worker's native script bytes.
    pub script: Vec<u8>,
}

struct SourceFilename {
    backend: Vec<u8>,
    reported: Vec<u8>,
    label: Vec<u8>,
}

fn filename_encoding_error(reported: &[u8]) -> CmdError {
    CmdError::with_error_code_bytes(
        [
            b"couldn't read file \"".as_slice(),
            reported,
            b"\": invalid or incomplete multibyte or wide character",
        ]
        .concat(),
        b"POSIX EILSEQ {invalid or incomplete multibyte or wide character}".to_vec(),
    )
}

fn external_iso88591_filename(
    version: TclVersion,
    original: &[u8],
    reported: &[u8],
) -> Result<Vec<u8>, CmdError> {
    let units = tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version);
    let mut external = Vec::with_capacity(original.len());
    let mut offset = 0;
    let mut previous = None;
    while let Some(unit) = units.decode_unit(&original[offset..], previous) {
        match u8::try_from(unit.value) {
            Ok(byte) => external.push(byte),
            Err(_) if version < TclVersion::V9_0 => external.push(b'?'),
            Err(_) => return Err(filename_encoding_error(reported)),
        }
        offset += unit.width;
        previous = Some(unit.value);
    }
    Ok(external)
}

fn prepare_source_filename<O: OriginalSourceFileOps>(
    ops: &mut O,
    protocol: NativeSourceFileProtocol,
    system: SystemEncoding,
    original: &[u8],
) -> Result<SourceFilename, CmdError> {
    // naming.source.original-counted-filename-filesystem-extent
    // docs/design/analysis/name-resolution-proofs/source-original-counted-filename-filesystem-extent.md
    let reported = tcl_core_types::c_string_extent(original).to_vec();
    if protocol == NativeSourceFileProtocol::Jim {
        return Ok(SourceFilename {
            backend: reported.clone(),
            label: reported.clone(),
            reported,
        });
    }
    let version = match protocol {
        NativeSourceFileProtocol::CSystem(version) | NativeSourceFileProtocol::CUtf8(version) => {
            version
        }
        NativeSourceFileProtocol::Jim => unreachable!("Jim filename returned separately"),
    };
    let converted: Result<Vec<u8>, CmdError> = match system {
        SystemEncoding::Utf8 => super::external_utf8_octets(version, original)
            .map_err(|_| filename_encoding_error(&reported)),
        SystemEncoding::Iso88591 => external_iso88591_filename(version, original, &reported),
        SystemEncoding::Identity => Ok(original.to_vec()),
        _ => {
            let units = tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version)
                .decode_units(original);
            ops.external_filename_bytes(&units, system)
        }
    };
    if let Err(error) = &converted {
        ops.retain_failure(error);
    }
    ops.check()?;
    let converted = converted?;
    let backend = if version == TclVersion::V8_4 {
        // The actual Unix worker applies CString extent AFTER external conversion.
        tcl_core_types::c_string_extent(&converted).to_vec()
    } else {
        if converted.contains(&0) {
            return Err(CmdError::with_error_code_bytes(
                [
                    b"couldn't read file \"".as_slice(),
                    &reported,
                    b"\": invalid argument",
                ]
                .concat(),
                b"POSIX EINVAL {invalid argument}".to_vec(),
            ));
        }
        converted
    };
    Ok(SourceFilename {
        backend,
        reported,
        label: original.to_vec(),
    })
}

/// Obtain encoding String, open the actual file, select encoding, read, and always close it.
/// Pure conversion uses existing channel decoding and native UTF representation workers.
///
/// # Errors
/// Returns full original stage refusal, actual file failure, or selected lookup failure.
pub fn prepare_source_file<O: OriginalSourceFileOps>(
    ops: &mut O,
    protocol: NativeSourceFileProtocol,
    path_at: usize,
    encoding_at: Option<usize>,
) -> Result<PreparedSourceFile, CmdError> {
    ops.check()?;
    let name = encoding_at
        .map(|index| ops.original_bytes(index))
        .transpose();
    ops.check()?;
    let name = name?;
    let path = ops.original_bytes(path_at);
    ops.check()?;
    let path = path?;
    let filename_system = ops.system_encoding();
    let filename = prepare_source_filename(ops, protocol, filename_system, &path)?;
    let default = ops.system_encoding();
    let path = filename.reported;
    let opened = ops.open_file(&filename.backend);
    let after_open = ops.check();
    let mut file = match opened {
        Ok(Ok(file)) => file,
        Ok(Err(error)) => {
            after_open?;
            return Err(protocol.read_error(&path, error));
        }
        Err(error) => {
            after_open?;
            return Err(error);
        }
    };
    let reached = (|| -> Result<PreparedSourceFile, CmdError> {
        after_open?;
        let encoding = match name {
            Some(name) => {
                let lookup = ops.lookup_encoding(
                    protocol
                        .encoding()
                        .ok_or(ValueError::CommandProtocolUnavailable(
                            "source encoding lookup",
                        ))?,
                    &name,
                );
                ops.check()?;
                lookup?
            }
            None if matches!(
                protocol,
                NativeSourceFileProtocol::CUtf8(_) | NativeSourceFileProtocol::Jim
            ) =>
            {
                ChannelEncoding::Utf8
            }
            None => ChannelEncoding::from_system(default),
        };
        let read = file.read_all();
        ops.check()?;
        let bytes = read.map_err(|error| protocol.read_error(&path, error))?;
        if protocol == NativeSourceFileProtocol::Jim {
            return Ok(PreparedSourceFile {
                path: filename.label.clone(),
                script: bytes,
            });
        }
        let script = if encoding == ChannelEncoding::Identity {
            bytes
        } else {
            let system = match encoding {
                ChannelEncoding::Utf8 => SystemEncoding::Utf8,
                ChannelEncoding::Iso88591 | ChannelEncoding::Tcl8Binary => SystemEncoding::Iso88591,
                ChannelEncoding::Ascii => SystemEncoding::Ascii,
                ChannelEncoding::Unicode => SystemEncoding::Unicode,
                ChannelEncoding::Identity => unreachable!("identity returned separately"),
            };
            let text = decode_text(&bytes, system);
            let version = match protocol {
                NativeSourceFileProtocol::CSystem(version)
                | NativeSourceFileProtocol::CUtf8(version) => version,
                NativeSourceFileProtocol::Jim => unreachable!("Jim returned separately"),
            };
            let units: Vec<u32> = if version < TclVersion::V9_0 {
                text.encode_utf16().map(u32::from).collect()
            } else {
                text.chars().map(u32::from).collect()
            };
            tcl_syntax::native_tcl_utf::NativeTclUtf::for_version(version)
                .encode_units(&units)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "source text conversion",
                ))?
        };
        ops.check()?;
        Ok(PreparedSourceFile {
            path: filename.label.clone(),
            script,
        })
    })();
    // Once open succeeded, the exact handle retires on every path, even Host refusal.
    if let Err(error) = &reached {
        ops.retain_failure(error);
    }
    let closed = file.close();
    // Native source ignores a close Guest result while preserving an earlier failed stage.
    let closed = if reached.is_ok() {
        closed.map_err(|error| protocol.read_error(&path, error))
    } else {
        Ok(())
    };
    if let Err(error) = &closed {
        ops.retain_failure(error);
    }
    ops.check()?;
    let prepared = reached?;
    closed?;
    Ok(prepared)
}

#[cfg(test)]
mod tests;
