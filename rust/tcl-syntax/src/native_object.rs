// SPDX-License-Identifier: AGPL-3.0-or-later
//! Physical object snapshots, independent of selected execution permission.
//!
//! The concrete object owner supplies these fields without materialization.
//! A pure snapshot grants no native engine, compiler, effect or identity proof.

use std::rc::Rc;

use crate::native_string::{NativeStringProtocol, NativeStringStorageIdentity};
use crate::scalar_getter::NativeScalarCache;
use tcl_dialect::TclVersion;

/// Independently retained primary object representation.
#[derive(Debug, Clone, PartialEq)]
pub enum NativeObjectCacheSnapshot {
    /// Parsed native level number; this cache owns no frame or interpreter.
    FrameReference {
        /// Actual original C descriptor release.
        version: TclVersion,
        /// C8.5 relative offsets are resolved against each current frame chain.
        relative: bool,
        /// Original signed parsed offset or absolute level.
        level: i32,
    },
    /// Actual original instruction-name primary; metadata grants no execution.
    InstructionName {
        /// Selected original C descriptor release.
        version: TclVersion,
        /// Native opcode stored in the original longValue payload.
        opcode: u8,
    },
    /// Fresh NULL internal representation, independently of resident bytes.
    None,
    /// Actual reached C native String internal representation.
    String {
        /// String-unit recipe retained when the representation was reached.
        protocol: NativeStringProtocol,
        /// Existing native character count, if known without conversion.
        num_chars: Option<usize>,
        /// Existing native Unicode units; absence is not an empty Unicode rep.
        unicode: Option<Rc<[u32]>>,
    },
    /// Actual pinned Jim String representation.
    JimString {
        /// Native stored character count; None corresponds to its unknown count.
        num_chars: Option<usize>,
    },
    /// Actual byte-array backing, independently of resident string bytes.
    ByteArray {
        /// Original native binary payload.
        bytes: Rc<[u8]>,
        /// Actual proper-byte-array eligibility, not inferred from its type name.
        proper: bool,
    },
    /// Complete current numeric or Jim coerced-integer cache.
    Numeric(NativeScalarCache),
    /// Actual word-Boolean descriptor, distinct from numeric Boolean values.
    WordBoolean {
        /// Retained Boolean value.
        value: bool,
        /// Original actual C descriptor release.
        version: TclVersion,
    },
    /// Original ordinary List backing.
    List {
        /// Actual backing length, without parsing or string generation.
        length: usize,
        /// Actual canonical flag retained in the primary List representation.
        canonical: bool,
    },
    /// Original ordinary Dictionary backing.
    Dictionary {
        /// Actual distinct-key count.
        size: usize,
        /// Actual absence of resident string storage.
        pure: bool,
    },
    /// Actual Index primary metadata; table authority stays on the original.
    Index {
        /// Actual C descriptor release.
        version: TclVersion,
        /// Selected native entry index.
        index: usize,
        /// Actual entry byte stride.
        stride: usize,
    },
    /// Actual named-command primary cache. This metadata grants no lookup hit
    /// or transportable command-token authority; original capabilities retain it.
    CommandName {
        /// Actual native C descriptor release.
        version: TclVersion,
        /// Whether the descriptor contains a resolved original command node.
        resolved: bool,
    },
    /// Actual namespace-name primary. This metadata grants no namespace hit
    /// or transportable token authority; the original object retains its descriptor.
    NamespaceName {
        /// Actual native C descriptor release.
        version: TclVersion,
        /// Whether the primary owns a resolved descriptor rather than C8.4 NULL.
        resolved: bool,
    },
    /// Actual pinned Jim command primary; metadata grants no live lookup.
    JimCommand {
        /// Native procedure epoch at original successful lookup.
        procedure_epoch: u64,
    },
    /// Actual pinned Jim variable primary; no cell or frame ownership.
    JimVariable {
        /// Original selected native frame incarnation.
        frame: u64,
        /// Whether lookup selects the actual top frame.
        global: bool,
    },
    /// Original Jim option table and exact flags (no pointer authority in a snapshot).
    JimEnum { flags: i32, index: usize },
    /// Original successful static literal comparison, no string updater.
    JimComparedString,

    /// Original Jim Script primary, independent of initialized line metadata.
    JimScript {
        /// Exact native substitution flags, zero for ordinary Script.
        flags: u8,
        /// Real retained token objects, including ordinary LINE/WORD entries.
        tokens: usize,
    },
    /// Actual owned original name/index tuple; metadata grants no lookup right.
    JimDictionarySubstitution,
    /// Actual borrowed name/owned index optimization, with no reconstructed name.
    JimInterpolated,
    /// Genuine C preparsed name; metadata grants no variable receiver authority.
    ParsedVariableName {
        /// Original native descriptor release.
        version: TclVersion,
        /// Whether the descriptor owns preparsed array parts.
        array: bool,
    },
    /// Genuine C compiled-local name descriptor; no transportable lookup grant.
    LocalVariableName {
        /// Original native descriptor release.
        version: TclVersion,
        /// Actual retained compiled-local index.
        index: usize,
    },
    /// Actual admitted C executable script retained by its original body object.
    Bytecode {
        /// Original selected native release; this snapshot grants no cache hit.
        version: TclVersion,
    },
    /// Another retained descriptor, without stock-class donation.
    Other,
}

/// Original storage and primary cache before any purpose-specific operation.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeObjectSnapshot {
    /// Exact existing string bytes; None remains physically unmaterialized.
    pub resident: Option<Rc<[u8]>>,
    /// Allocation identity of the existing string; None means no string exists.
    pub storage: Option<NativeStringStorageIdentity>,
    /// Independently retained primary representation.
    pub cache: NativeObjectCacheSnapshot,
}

/// Native C9 empty-string enquiry without invoking an object updater.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeObjectStringEmptiness {
    /// The selected native shape proves an empty string.
    Empty,
    /// The selected native shape proves a nonempty string.
    Nonempty,
    /// Native C cannot determine emptiness without generating a string.
    Unknown,
}

/// Missing physical information needed by a selected object-shape recipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeObjectShapeUnavailable {
    /// Canonical storage identity would change this enquiry's result.
    EmptyStorageIdentity,
    /// The snapshot's resident/storage fields cannot describe a physical object.
    InconsistentStorage,
}

/// Apply the exact C9 empty-string shape recipe before materialisation.
/// This pure helper supplies no actual-engine authority to its caller.
///
/// # Errors
/// Refuses inconsistent storage or a missing identity that changes the result.
pub fn native_c9_string_emptiness(
    snapshot: &NativeObjectSnapshot,
) -> Result<NativeObjectStringEmptiness, NativeObjectShapeUnavailable> {
    native_c_string_emptiness(TclVersion::V9_0, snapshot)
}

/// Apply the selected C8.6+ empty enquiry without generating original bytes.
pub fn native_c_string_emptiness(
    version: TclVersion,
    snapshot: &NativeObjectSnapshot,
) -> Result<NativeObjectStringEmptiness, NativeObjectShapeUnavailable> {
    use crate::native_string::NativeStringStorageIdentity as Storage;
    use NativeObjectStringEmptiness::{Empty, Nonempty, Unknown};
    match (&snapshot.resident, snapshot.storage) {
        (None, Some(_)) | (Some(_), None) => {
            return Err(NativeObjectShapeUnavailable::InconsistentStorage);
        }
        (Some(bytes), Some(Storage::CanonicalEmpty)) if !bytes.is_empty() => {
            return Err(NativeObjectShapeUnavailable::InconsistentStorage);
        }
        _ => {}
    }
    let without_canonical = match &snapshot.cache {
        NativeObjectCacheSnapshot::ByteArray {
            bytes,
            proper: true,
        } if version >= TclVersion::V9_0 && bytes.is_empty() => Empty,
        NativeObjectCacheSnapshot::List { length, canonical }
            if (version >= TclVersion::V9_0 && *canonical) || snapshot.resident.is_none() =>
        {
            if *length == 0 {
                Empty
            } else {
                Nonempty
            }
        }
        NativeObjectCacheSnapshot::Dictionary { size, pure: true } => {
            if *size == 0 {
                Empty
            } else {
                Nonempty
            }
        }
        _ => snapshot.resident.as_ref().map_or(Unknown, |bytes| {
            if bytes.is_empty() { Empty } else { Nonempty }
        }),
    };
    match snapshot.storage {
        Some(Storage::CanonicalEmpty) => Ok(Empty),
        Some(Storage::Unknown)
            if snapshot
                .resident
                .as_ref()
                .is_some_and(|bytes| bytes.is_empty())
                && without_canonical != Empty =>
        {
            Err(NativeObjectShapeUnavailable::EmptyStorageIdentity)
        }
        _ => Ok(without_canonical),
    }
}
