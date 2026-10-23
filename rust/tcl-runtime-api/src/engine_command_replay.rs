// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fresh command setup data from actual namespace holders, without live identity.

use tcl_core_types::{ByteNamespacePath, NameBytes};
use tcl_dialect::TclVersion;
use tcl_syntax::naming::NativeNameProtocol;

/// The existing counted operation to repeat in a fresh interpreter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineCommandReplayPurpose {
    /// Counted C object-command publication through its selected recipe.
    Create,
    /// Original command lookup followed by retirement.
    Remove,
}

/// Namespace data retains C component boundaries or Jim's counted holder bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineCommandReplayNamespace {
    /// Actual C constructed edges and live command-path holders at capture.
    C {
        /// Current namespace's exact component geometry.
        namespace: ByteNamespacePath,
        /// A retired source holder cannot be recreated as a live holder.
        live: bool,
        /// Ordered live command-search holders in the source context.
        command_path: Vec<ByteNamespacePath>,
    },
    /// Actual canonical Jim namespace object bytes; not a C component path.
    Jim(NameBytes),
}

/// Portable replay data grants neither a native operation nor an old generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineCommandReplayContext {
    /// Actual source was a root interpreter, not a child factory's receiver.
    pub root_interpreter: bool,
    /// Exact selected original naming recipe.
    pub protocol: NativeNameProtocol,
    /// Independently selected operation purpose.
    pub purpose: EngineCommandReplayPurpose,
    /// Actual holder geometry, never parsed from a displayed command name.
    pub namespace: EngineCommandReplayNamespace,
}

impl EngineCommandReplayContext {
    /// Encode counted components independently of Rust layout or Tcl list syntax.
    #[must_use]
    pub fn encode(&self) -> Option<Vec<u8>> {
        let mut out = b"NCR1".to_vec();
        out.push(u8::from(self.root_interpreter));
        out.push(match self.protocol {
            NativeNameProtocol::C(TclVersion::V8_4) => 1,
            NativeNameProtocol::C(TclVersion::V8_5) => 2,
            NativeNameProtocol::C(TclVersion::V8_6) => 3,
            NativeNameProtocol::C(TclVersion::V9_0) => 4,
            NativeNameProtocol::C(TclVersion::V9_1) => 5,
            NativeNameProtocol::Jim084 => 6,
        });
        out.push(match self.purpose {
            EngineCommandReplayPurpose::Create => 1,
            EngineCommandReplayPurpose::Remove => 2,
        });
        match &self.namespace {
            EngineCommandReplayNamespace::C {
                namespace,
                live,
                command_path,
            } => {
                if self.protocol.is_jim084() {
                    return None;
                }
                out.push(u8::from(*live));
                write_path(&mut out, namespace)?;
                write_count(&mut out, command_path.len())?;
                for path in command_path {
                    write_path(&mut out, path)?;
                }
            }
            EngineCommandReplayNamespace::Jim(namespace) => {
                if !self.protocol.is_jim084() {
                    return None;
                }
                write_bytes(&mut out, namespace.as_bytes())?;
            }
        }
        Some(out)
    }

    /// Decode data only. The receiving runtime authenticates its own operation.
    #[must_use]
    pub fn decode(mut bytes: &[u8]) -> Option<Self> {
        if take(&mut bytes, 4)? != b"NCR1" {
            return None;
        }
        let root_interpreter = match take(&mut bytes, 1)?[0] {
            0 => false,
            1 => true,
            _ => return None,
        };
        let protocol = match take(&mut bytes, 1)?[0] {
            1 => NativeNameProtocol::C(TclVersion::V8_4),
            2 => NativeNameProtocol::C(TclVersion::V8_5),
            3 => NativeNameProtocol::C(TclVersion::V8_6),
            4 => NativeNameProtocol::C(TclVersion::V9_0),
            5 => NativeNameProtocol::C(TclVersion::V9_1),
            6 => NativeNameProtocol::Jim084,
            _ => return None,
        };
        let purpose = match take(&mut bytes, 1)?[0] {
            1 => EngineCommandReplayPurpose::Create,
            2 => EngineCommandReplayPurpose::Remove,
            _ => return None,
        };
        let namespace = if protocol.is_jim084() {
            EngineCommandReplayNamespace::Jim(NameBytes::from(read_bytes(&mut bytes)?))
        } else {
            let live = match take(&mut bytes, 1)?[0] {
                0 => false,
                1 => true,
                _ => return None,
            };
            let namespace = read_path(&mut bytes)?;
            let count = read_count(&mut bytes)?;
            let mut command_path = Vec::new();
            for _ in 0..count {
                command_path.push(read_path(&mut bytes)?);
            }
            EngineCommandReplayNamespace::C {
                namespace,
                live,
                command_path,
            }
        };
        bytes.is_empty().then_some(Self {
            root_interpreter,
            protocol,
            purpose,
            namespace,
        })
    }
}

fn write_count(out: &mut Vec<u8>, count: usize) -> Option<()> {
    out.extend_from_slice(&u32::try_from(count).ok()?.to_le_bytes());
    Some(())
}
fn write_bytes(out: &mut Vec<u8>, bytes: &[u8]) -> Option<()> {
    write_count(out, bytes.len())?;
    out.extend_from_slice(bytes);
    Some(())
}
fn write_path(out: &mut Vec<u8>, path: &ByteNamespacePath) -> Option<()> {
    write_count(out, path.as_segments().len())?;
    for part in path.as_segments() {
        write_bytes(out, part.as_bytes())?;
    }
    Some(())
}
fn take<'a>(bytes: &mut &'a [u8], count: usize) -> Option<&'a [u8]> {
    let (taken, remaining) = bytes.split_at_checked(count)?;
    *bytes = remaining;
    Some(taken)
}
fn read_count(bytes: &mut &[u8]) -> Option<usize> {
    usize::try_from(u32::from_le_bytes(take(bytes, 4)?.try_into().ok()?)).ok()
}
fn read_bytes<'a>(bytes: &mut &'a [u8]) -> Option<&'a [u8]> {
    let count = read_count(bytes)?;
    take(bytes, count)
}
fn read_path(bytes: &mut &[u8]) -> Option<ByteNamespacePath> {
    let count = read_count(bytes)?;
    let mut parts = Vec::new();
    for _ in 0..count {
        parts.push(NameBytes::from(read_bytes(bytes)?));
    }
    Some(ByteNamespacePath::from_segments(parts))
}
