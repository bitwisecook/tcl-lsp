// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! What an artefact says about the world it was compiled for, and what a
//! runtime holds to compare that against —
//! `docs/design/compiler/registry-consumer-contracts.md` § *Artefacts carry an
//! identity manifest*.
//!
//! The artefact and the runtime state the same thing in the same shape: a
//! [`RuntimeContext`] (environment, release, build, package floors, overlay
//! generation) made into an [`ArtefactIdentityManifest`] by
//! [`RuntimeContext::identity`]. The compiler does it for the context it
//! compiled under, plus the pack facts its sites rest on; a runtime does it
//! for the context it is pinned to, plus the pack facts it holds. The
//! manifest is checked as a whole ([`ArtefactIdentityManifest::disagreements`]),
//! and a field that disagrees refuses the [`Rung`]s that rest on it
//! ([`ManifestField::rests_on`]) and no others, so a unit with only rung-0
//! sites is admitted under a changed pack set.

use std::fmt::Write as _;

use tcl_dialect::model::BuildProfileId;

use crate::PackFactStamp;
use crate::codegen_abi::CODEGEN_ABI_VERSION;

/// The name of the WASM custom section a module carries its manifest in.
pub const WASM_SECTION: &str = "tcl.manifest";

/// The revision of the Tcl library both runtimes embed: the patchlevel, the
/// upstream commit it was vendored from (twelve characters) and a digest of
/// the embedded files' hashes (twelve characters), as
/// `runtime/rust/vendor/tcl_library/manifest.json` holds them. `cargo xtask
/// runtime-stdlib` holds this equal to the manifest's, so a refreshed or
/// patched library cannot leave the revision where it was.
pub const EMBEDDED_STDLIB_REVISION: &str = "9.0.4+c655b4770b1d.86f5852aba2f";

/// The number of fields a manifest lists.
const FIELD_COUNT: usize = 8;

/// One rung of the codegen ladder, as a site's record places it.
///
/// Rung 0 is generic dispatch, which records nothing and which every function
/// has; the others are the records a specialised site carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rung {
    /// Rung 0: generic dispatch, and the decoding every unit's words rest on.
    Generic = 0,
    /// Rung 1: a constant a pack's `const_fold` computed.
    PackFacts = 1,
    /// Rung 2: a builtin reached through a pack command's `alias_of`.
    BuiltinAlias = 2,
    /// Rung 3: an exact user-procedure body copied into the site.
    ReferenceBody = 3,
    /// Rung 4: a specialisation that rests on a shipped implementation's
    /// identity.
    ShippedBacking = 4,
}

/// A set of [`Rung`]s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RungSet(u8);

impl RungSet {
    /// No rung.
    pub const EMPTY: Self = Self(0);
    /// Every rung: a refusal of the whole unit.
    pub const ALL: Self = Self(0b1_1111);

    /// The set holding `rung` alone.
    #[must_use]
    pub const fn of(rung: Rung) -> Self {
        Self(1 << rung as u8)
    }

    /// This set and `rung`.
    #[must_use]
    pub const fn with(self, rung: Rung) -> Self {
        Self(self.0 | 1 << rung as u8)
    }

    /// Every rung of either set.
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Whether `rung` is in the set.
    #[must_use]
    pub const fn contains(self, rung: Rung) -> bool {
        self.0 & (1 << rung as u8) != 0
    }

    /// Whether the two sets share a rung.
    #[must_use]
    pub const fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Whether the set is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// One field of an [`ArtefactIdentityManifest`], in declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ManifestField {
    /// [`ArtefactIdentityManifest::abi_version`].
    AbiVersion,
    /// [`ArtefactIdentityManifest::environment`].
    Environment,
    /// [`ArtefactIdentityManifest::release`].
    Release,
    /// [`ArtefactIdentityManifest::build`].
    Build,
    /// [`ArtefactIdentityManifest::packages`].
    Packages,
    /// [`ArtefactIdentityManifest::packs`].
    Packs,
    /// [`ArtefactIdentityManifest::intrinsic_table_hash`].
    IntrinsicTableHash,
    /// [`ArtefactIdentityManifest::embedded_stdlib_revision`].
    EmbeddedStdlibRevision,
}

impl ManifestField {
    /// The field's name, as the manifest spells it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::AbiVersion => "abi_version",
            Self::Environment => "environment",
            Self::Release => "release",
            Self::Build => "build",
            Self::Packages => "packages",
            Self::Packs => "packs",
            Self::IntrinsicTableHash => "intrinsic_table_hash",
            Self::EmbeddedStdlibRevision => "embedded_stdlib_revision",
        }
    }

    /// The rungs whose sites rest on this field, which a disagreement on it
    /// refuses.
    ///
    /// The ABI and the world a unit was lexed and specialised for decide what
    /// every site means, generic dispatch included: a string constant a unit
    /// decoded under one release's escapes is not the constant another
    /// release's would. The pack facts and the package floors are what rungs 1
    /// and 2 rest on. The intrinsic table and the embedded library are what a
    /// specialisation resting on a shipped implementation's identity assumes.
    #[must_use]
    pub const fn rests_on(self) -> RungSet {
        match self {
            Self::AbiVersion | Self::Environment | Self::Release | Self::Build => RungSet::ALL,
            Self::Packages | Self::Packs => RungSet::of(Rung::PackFacts).with(Rung::BuiltinAlias),
            Self::IntrinsicTableHash | Self::EmbeddedStdlibRevision => {
                RungSet::of(Rung::ShippedBacking)
            }
        }
    }
}

/// What an artefact says about the world it was compiled for.
///
/// On bytecode it rides on the compiled unit, beside the generations the VM
/// already records; on a WASM module it is the custom section
/// [`WASM_SECTION`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtefactIdentityManifest {
    /// The runtime ABI the module's imports were emitted against:
    /// [`CODEGEN_ABI_VERSION`], the fingerprint of `CodegenAbiImportId`'s table.
    pub abi_version: u32,
    /// The resolved environment id, as the ingress interns it.
    pub environment: String,
    /// The release point within that environment, so a per-target evaluation
    /// is re-checkable.
    pub release: String,
    /// The build profile the environment resolved to.
    pub build: BuildProfileId,
    /// Package floors in force at compile time, name and version, by name.
    pub packages: Vec<(String, String)>,
    /// One entry per pack any site rested on — the `SiteClaim` stamps,
    /// deduplicated.
    pub packs: Vec<PackFactStamp>,
    /// The intrinsic table the emitter keyed against, so a runtime whose table
    /// differs refuses rather than mis-dispatches.
    pub intrinsic_table_hash: [u8; 32],
    /// The embedded stdlib revision the unit's `source` route assumed.
    pub embedded_stdlib_revision: String,
}

/// The world a runtime is pinned to, or a compile ran under: the
/// environment, the release point within it, the build, the package floors in
/// force, and the registry overlay generation.
///
/// Resolved through the same ingress the compiler uses
/// (`tcl_registry::model::runtime_context`), where an overlay nothing has
/// installed is an error and never the un-overlaid generation under another
/// name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeContext {
    /// The environment's canonical id.
    pub environment: String,
    /// The release point's spelling (`"8.6"`), empty for an environment with no
    /// Tcl ladder.
    pub release: String,
    /// The build profile of that point.
    pub build: BuildProfileId,
    /// The package floors in force, name and version, by name.
    pub packages: Vec<(String, String)>,
    /// The registry overlay generation the packs were installed under; `0` is
    /// none.
    pub overlay_generation: u64,
}

impl RuntimeContext {
    /// The identity this context states, in the shape an artefact states its
    /// own: the context's fields, the pack facts (sorted and without repeats),
    /// the intrinsic-table hash, and this build's ABI version and embedded
    /// library revision.
    #[must_use]
    pub fn identity(
        &self,
        packs: &[PackFactStamp],
        intrinsic_table_hash: [u8; 32],
    ) -> ArtefactIdentityManifest {
        let mut packs = packs.to_vec();
        packs.sort();
        packs.dedup();
        ArtefactIdentityManifest {
            abi_version: CODEGEN_ABI_VERSION,
            environment: self.environment.clone(),
            release: self.release.clone(),
            build: self.build,
            packages: self.packages.clone(),
            packs,
            intrinsic_table_hash,
            embedded_stdlib_revision: EMBEDDED_STDLIB_REVISION.to_owned(),
        }
    }
}

impl ArtefactIdentityManifest {
    /// The fields in which this manifest, an artefact's statement, disagrees
    /// with `held`, a runtime's: each field must be equal, except the packs,
    /// every one of which the runtime must hold.
    #[must_use]
    pub fn disagreements(&self, held: &Self) -> Vec<ManifestField> {
        let mut fields = Vec::new();
        let mut note = |field: ManifestField, disagrees: bool| {
            if disagrees {
                fields.push(field);
            }
        };
        note(
            ManifestField::AbiVersion,
            self.abi_version != held.abi_version,
        );
        note(
            ManifestField::Environment,
            self.environment != held.environment,
        );
        note(ManifestField::Release, self.release != held.release);
        note(ManifestField::Build, self.build != held.build);
        note(ManifestField::Packages, self.packages != held.packages);
        note(
            ManifestField::Packs,
            !self.packs.iter().all(|stamp| held.packs.contains(stamp)),
        );
        note(
            ManifestField::IntrinsicTableHash,
            self.intrinsic_table_hash != held.intrinsic_table_hash,
        );
        note(
            ManifestField::EmbeddedStdlibRevision,
            self.embedded_stdlib_revision != held.embedded_stdlib_revision,
        );
        fields
    }

    /// The rungs a disagreement with `held` refuses.
    #[must_use]
    pub fn refused_rungs(&self, held: &Self) -> RungSet {
        self.disagreements(held)
            .into_iter()
            .fold(RungSet::EMPTY, |refused, field| {
                refused.union(field.rests_on())
            })
    }

    /// `field`'s value as a message spells it.
    #[must_use]
    pub fn describe(&self, field: ManifestField) -> String {
        match field {
            ManifestField::AbiVersion => self.abi_version.to_string(),
            ManifestField::Environment => self.environment.clone(),
            ManifestField::Release => self.release.clone(),
            ManifestField::Build => format!("{:?}", self.build),
            ManifestField::Packages => self
                .packages
                .iter()
                .map(|(name, version)| format!("{name} {version}"))
                .collect::<Vec<_>>()
                .join(", "),
            ManifestField::Packs => self
                .packs
                .iter()
                .map(|stamp| format!("{}@{:x}", stamp.pack, stamp.content_hash))
                .collect::<Vec<_>>()
                .join(", "),
            ManifestField::IntrinsicTableHash => {
                self.intrinsic_table_hash
                    .iter()
                    .fold(String::new(), |mut hex, byte| {
                        let _ = write!(hex, "{byte:02x}");
                        hex
                    })
            }
            ManifestField::EmbeddedStdlibRevision => self.embedded_stdlib_revision.clone(),
        }
    }
}

/// Why a manifest's bytes did not decode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestDecodeError {
    /// The bytes ended inside a field.
    Truncated,
    /// The list held another number of fields than a manifest lists.
    FieldCount(usize),
    /// A field held bytes past its own end.
    Trailing(&'static str),
    /// A field held something its type does not: text that is not UTF-8, a
    /// build code nothing names.
    Malformed(&'static str),
    /// The module's own structure did not parse as far as its sections.
    NotAModule,
}

impl std::fmt::Display for ManifestDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated => f.write_str("the manifest ends inside a field"),
            Self::FieldCount(found) => {
                write!(f, "the manifest lists {found} fields, not {FIELD_COUNT}")
            }
            Self::Trailing(field) => write!(f, "the manifest's {field} has bytes past its end"),
            Self::Malformed(field) => write!(f, "the manifest's {field} is malformed"),
            Self::NotAModule => f.write_str("the bytes are not a WASM module"),
        }
    }
}

impl std::error::Error for ManifestDecodeError {}

const fn build_code(build: BuildProfileId) -> u8 {
    match build {
        BuildProfileId::Canonical => 0,
        BuildProfileId::JimFull => 1,
        BuildProfileId::JimMinimal => 2,
        BuildProfileId::F5Scriptd32 => 3,
        BuildProfileId::Unknown => 4,
    }
}

const fn build_of_code(code: u8) -> Option<BuildProfileId> {
    match code {
        0 => Some(BuildProfileId::Canonical),
        1 => Some(BuildProfileId::JimFull),
        2 => Some(BuildProfileId::JimMinimal),
        3 => Some(BuildProfileId::F5Scriptd32),
        4 => Some(BuildProfileId::Unknown),
        _ => None,
    }
}

fn put_len(out: &mut Vec<u8>, len: usize) {
    let len = u32::try_from(len).expect("a manifest field is shorter than 4 GiB");
    out.extend_from_slice(&len.to_le_bytes());
}

fn put_text(out: &mut Vec<u8>, text: &str) {
    put_len(out, text.len());
    out.extend_from_slice(text.as_bytes());
}

fn put_field(out: &mut Vec<u8>, field: &[u8]) {
    put_len(out, field.len());
    out.extend_from_slice(field);
}

struct Cursor<'a>(&'a [u8]);

impl<'a> Cursor<'a> {
    fn take(&mut self, len: usize) -> Result<&'a [u8], ManifestDecodeError> {
        if self.0.len() < len {
            return Err(ManifestDecodeError::Truncated);
        }
        let (head, rest) = self.0.split_at(len);
        self.0 = rest;
        Ok(head)
    }

    fn u32(&mut self) -> Result<u32, ManifestDecodeError> {
        let bytes: [u8; 4] = self.take(4)?.try_into().expect("four bytes were taken");
        Ok(u32::from_le_bytes(bytes))
    }

    fn u64(&mut self) -> Result<u64, ManifestDecodeError> {
        let bytes: [u8; 8] = self.take(8)?.try_into().expect("eight bytes were taken");
        Ok(u64::from_le_bytes(bytes))
    }

    fn len(&mut self) -> Result<usize, ManifestDecodeError> {
        Ok(self.u32()? as usize)
    }

    fn text(&mut self, field: &'static str) -> Result<String, ManifestDecodeError> {
        let len = self.len()?;
        String::from_utf8(self.take(len)?.to_vec())
            .map_err(|_| ManifestDecodeError::Malformed(field))
    }

    fn field(&mut self) -> Result<&'a [u8], ManifestDecodeError> {
        let len = self.len()?;
        self.take(len)
    }

    fn finish(&self, field: &'static str) -> Result<(), ManifestDecodeError> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(ManifestDecodeError::Trailing(field))
        }
    }
}

impl ArtefactIdentityManifest {
    /// The manifest as a length-prefixed field list in declaration order: each
    /// field a little-endian `u32` byte length and its bytes. Integers are
    /// little-endian; text is UTF-8 with a `u32` length; a list is a `u32`
    /// count and its items.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        put_field(&mut out, &self.abi_version.to_le_bytes());
        put_field(&mut out, self.environment.as_bytes());
        put_field(&mut out, self.release.as_bytes());
        put_field(&mut out, &[build_code(self.build)]);

        let mut packages = Vec::new();
        put_len(&mut packages, self.packages.len());
        for (name, version) in &self.packages {
            put_text(&mut packages, name);
            put_text(&mut packages, version);
        }
        put_field(&mut out, &packages);

        let mut packs = Vec::new();
        put_len(&mut packs, self.packs.len());
        for stamp in &self.packs {
            put_text(&mut packs, &stamp.pack);
            packs.extend_from_slice(&stamp.content_hash.to_le_bytes());
            put_text(&mut packs, &stamp.vocabulary_version);
            packs.extend_from_slice(&stamp.overlay_generation.to_le_bytes());
            packs.extend_from_slice(&stamp.evaluator_revision.to_le_bytes());
        }
        put_field(&mut out, &packs);

        put_field(&mut out, &self.intrinsic_table_hash);
        put_field(&mut out, self.embedded_stdlib_revision.as_bytes());
        out
    }

    /// The manifest [`Self::to_bytes`] wrote.
    ///
    /// # Errors
    ///
    /// [`ManifestDecodeError`] when the bytes are not exactly a manifest:
    /// another number of fields, a field cut short or longer than its type, text
    /// that is not UTF-8, or a build nothing names.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ManifestDecodeError> {
        let mut list = Cursor(bytes);
        let mut fields = Vec::new();
        while !list.0.is_empty() {
            fields.push(list.field()?);
        }
        let fields: [&[u8]; FIELD_COUNT] = fields
            .try_into()
            .map_err(|fields: Vec<&[u8]>| ManifestDecodeError::FieldCount(fields.len()))?;
        let [
            abi,
            environment,
            release,
            build,
            packages,
            packs,
            hash,
            stdlib,
        ] = fields;

        let mut cursor = Cursor(abi);
        let abi_version = cursor.u32()?;
        cursor.finish("abi_version")?;

        let environment = text_of(environment, "environment")?;
        let release = text_of(release, "release")?;
        let build = match build {
            [code] => build_of_code(*code).ok_or(ManifestDecodeError::Malformed("build"))?,
            _ => return Err(ManifestDecodeError::Malformed("build")),
        };

        let mut cursor = Cursor(packages);
        let mut pairs = Vec::new();
        for _ in 0..cursor.len()? {
            pairs.push((cursor.text("packages")?, cursor.text("packages")?));
        }
        cursor.finish("packages")?;

        let mut cursor = Cursor(packs);
        let mut stamps = Vec::new();
        for _ in 0..cursor.len()? {
            stamps.push(PackFactStamp {
                pack: cursor.text("packs")?,
                content_hash: cursor.u64()?,
                vocabulary_version: cursor.text("packs")?,
                overlay_generation: cursor.u64()?,
                evaluator_revision: cursor.u64()?,
            });
        }
        cursor.finish("packs")?;

        let intrinsic_table_hash: [u8; 32] = hash
            .try_into()
            .map_err(|_| ManifestDecodeError::Malformed("intrinsic_table_hash"))?;

        Ok(Self {
            abi_version,
            environment,
            release,
            build,
            packages: pairs,
            packs: stamps,
            intrinsic_table_hash,
            embedded_stdlib_revision: text_of(stdlib, "embedded_stdlib_revision")?,
        })
    }

    /// The manifest a WASM module carries in its [`WASM_SECTION`] custom
    /// section, `None` for a module with no such section.
    ///
    /// # Errors
    ///
    /// [`ManifestDecodeError::NotAModule`] when `module` does not parse as far
    /// as its section list, or another error when the section is not a
    /// manifest.
    pub fn from_wasm(module: &[u8]) -> Result<Option<Self>, ManifestDecodeError> {
        let Some(mut rest) = module.strip_prefix(b"\0asm\x01\0\0\0") else {
            return Err(ManifestDecodeError::NotAModule);
        };
        while !rest.is_empty() {
            let id = rest[0];
            let (size, after) = leb_u32(&rest[1..]).ok_or(ManifestDecodeError::NotAModule)?;
            if after.len() < size as usize {
                return Err(ManifestDecodeError::NotAModule);
            }
            let (content, next) = after.split_at(size as usize);
            rest = next;
            if id != 0 {
                continue;
            }
            let (name_len, name_and_payload) =
                leb_u32(content).ok_or(ManifestDecodeError::NotAModule)?;
            if name_and_payload.len() < name_len as usize {
                return Err(ManifestDecodeError::NotAModule);
            }
            let (name, payload) = name_and_payload.split_at(name_len as usize);
            if name == WASM_SECTION.as_bytes() {
                return Self::from_bytes(payload).map(Some);
            }
        }
        Ok(None)
    }
}

fn text_of(bytes: &[u8], field: &'static str) -> Result<String, ManifestDecodeError> {
    String::from_utf8(bytes.to_vec()).map_err(|_| ManifestDecodeError::Malformed(field))
}

/// An unsigned LEB128 from the front of `bytes`, and what follows it.
fn leb_u32(bytes: &[u8]) -> Option<(u32, &[u8])> {
    let mut value = 0u32;
    for (index, byte) in bytes.iter().enumerate().take(5) {
        value |= u32::from(byte & 0x7f).checked_shl(7 * u32::try_from(index).ok()?)?;
        if byte & 0x80 == 0 {
            return Some((value, &bytes[index + 1..]));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stamp(pack: &str, content_hash: u64) -> PackFactStamp {
        PackFactStamp {
            pack: pack.to_owned(),
            content_hash,
            vocabulary_version: "2".to_owned(),
            overlay_generation: 9,
            evaluator_revision: 3,
        }
    }

    fn context() -> RuntimeContext {
        RuntimeContext {
            environment: "tcl8.6".to_owned(),
            release: "8.6".to_owned(),
            build: BuildProfileId::Canonical,
            packages: vec![("Tk".to_owned(), "8.6".to_owned())],
            overlay_generation: 0,
        }
    }

    fn manifest() -> ArtefactIdentityManifest {
        context().identity(&[stamp("b", 2), stamp("a", 1)], [7; 32])
    }

    type Edit = fn(&mut ArtefactIdentityManifest);

    #[test]
    fn a_manifest_round_trips_through_its_bytes() {
        let manifest = manifest();
        assert_eq!(
            ArtefactIdentityManifest::from_bytes(&manifest.to_bytes()),
            Ok(manifest.clone())
        );
        let bare = context().identity(&[], [0; 32]);
        assert_eq!(
            ArtefactIdentityManifest::from_bytes(&bare.to_bytes()),
            Ok(bare)
        );
        for build in [
            BuildProfileId::Canonical,
            BuildProfileId::JimFull,
            BuildProfileId::JimMinimal,
            BuildProfileId::F5Scriptd32,
            BuildProfileId::Unknown,
        ] {
            let mut manifest = manifest.clone();
            manifest.build = build;
            assert_eq!(
                ArtefactIdentityManifest::from_bytes(&manifest.to_bytes()),
                Ok(manifest)
            );
        }
    }

    #[test]
    fn the_fields_are_length_prefixed_in_declaration_order() {
        let manifest = ArtefactIdentityManifest {
            abi_version: 0x0403_0201,
            environment: "e".to_owned(),
            release: "r".to_owned(),
            build: BuildProfileId::JimFull,
            packages: Vec::new(),
            packs: Vec::new(),
            intrinsic_table_hash: [0xab; 32],
            embedded_stdlib_revision: "v".to_owned(),
        };
        let mut expected: Vec<u8> = Vec::new();
        expected.extend([4, 0, 0, 0, 1, 2, 3, 4]);
        expected.extend([1, 0, 0, 0, b'e']);
        expected.extend([1, 0, 0, 0, b'r']);
        expected.extend([1, 0, 0, 0, 1]);
        expected.extend([4, 0, 0, 0, 0, 0, 0, 0]);
        expected.extend([4, 0, 0, 0, 0, 0, 0, 0]);
        expected.extend([32, 0, 0, 0]);
        expected.extend([0xab; 32]);
        expected.extend([1, 0, 0, 0, b'v']);
        assert_eq!(manifest.to_bytes(), expected);
    }

    #[test]
    fn bytes_that_are_not_exactly_a_manifest_do_not_decode() {
        let bytes = manifest().to_bytes();
        for cut in 0..bytes.len() {
            assert!(
                ArtefactIdentityManifest::from_bytes(&bytes[..cut]).is_err(),
                "a manifest cut at {cut} decoded"
            );
        }
        let mut longer = bytes.clone();
        longer.extend([0, 0, 0, 0]);
        assert_eq!(
            ArtefactIdentityManifest::from_bytes(&longer),
            Err(ManifestDecodeError::FieldCount(FIELD_COUNT + 1))
        );
        let known = manifest();
        let mut raw = known.to_bytes();
        let at = (4 + 4) + (4 + known.environment.len()) + (4 + known.release.len()) + 4;
        assert_eq!(raw[at], 0, "the build code is where this test looks");
        raw[at] = 9;
        assert_eq!(
            ArtefactIdentityManifest::from_bytes(&raw),
            Err(ManifestDecodeError::Malformed("build"))
        );
    }

    /// The bytes of `manifest` with `extra` appended to its `index`th field.
    fn with_extra_byte_in_field(manifest: &ArtefactIdentityManifest, index: usize) -> Vec<u8> {
        let bytes = manifest.to_bytes();
        let mut out = Vec::new();
        let mut at = 0;
        for field in 0..FIELD_COUNT {
            let len = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
            let body = &bytes[at + 4..at + 4 + len];
            at += 4 + len;
            if field == index {
                out.extend(u32::try_from(len + 1).unwrap().to_le_bytes());
                out.extend(body);
                out.push(0);
            } else {
                out.extend(u32::try_from(len).unwrap().to_le_bytes());
                out.extend(body);
            }
        }
        out
    }

    #[test]
    fn a_field_longer_than_its_type_does_not_decode() {
        let manifest = manifest();
        // abi_version, build, packages, packs and the hash each have a length
        // their type fixes or their own list ends at.
        for (index, refusal) in [
            (0, ManifestDecodeError::Trailing("abi_version")),
            (3, ManifestDecodeError::Malformed("build")),
            (4, ManifestDecodeError::Trailing("packages")),
            (5, ManifestDecodeError::Trailing("packs")),
            (6, ManifestDecodeError::Malformed("intrinsic_table_hash")),
        ] {
            assert_eq!(
                ArtefactIdentityManifest::from_bytes(&with_extra_byte_in_field(&manifest, index)),
                Err(refusal),
                "field {index}"
            );
        }
    }

    #[test]
    fn the_packs_are_sorted_and_without_repeats() {
        let manifest = context().identity(&[stamp("b", 2), stamp("a", 1), stamp("b", 2)], [0; 32]);
        assert_eq!(manifest.packs, vec![stamp("a", 1), stamp("b", 2)]);
    }

    #[test]
    fn each_field_that_disagrees_is_named_and_none_other() {
        let held = manifest();
        assert!(manifest().disagreements(&held).is_empty());
        let cases: [(ManifestField, Edit); 8] = [
            (ManifestField::AbiVersion, |m| m.abi_version += 1),
            (ManifestField::Environment, |m| {
                m.environment = "tcl9.0".to_owned();
            }),
            (ManifestField::Release, |m| m.release = "9.0".to_owned()),
            (ManifestField::Build, |m| m.build = BuildProfileId::Unknown),
            (ManifestField::Packages, |m| {
                m.packages.push(("X".to_owned(), "1".to_owned()));
            }),
            (ManifestField::Packs, |m| m.packs.push(stamp("c", 3))),
            (ManifestField::IntrinsicTableHash, |m| {
                m.intrinsic_table_hash[0] ^= 1;
            }),
            (ManifestField::EmbeddedStdlibRevision, |m| {
                m.embedded_stdlib_revision.push('x');
            }),
        ];
        for (field, edit) in cases {
            let mut artefact = manifest();
            edit(&mut artefact);
            assert_eq!(artefact.disagreements(&held), vec![field], "{field:?}");
        }
    }

    #[test]
    fn a_runtime_may_hold_more_packs_than_the_artefact_rests_on() {
        let held = context().identity(&[stamp("a", 1), stamp("b", 2), stamp("c", 3)], [7; 32]);
        assert!(manifest().disagreements(&held).is_empty());
        let fewer = context().identity(&[stamp("a", 1)], [7; 32]);
        assert_eq!(manifest().disagreements(&fewer), vec![ManifestField::Packs]);
        let changed = context().identity(&[stamp("a", 1), stamp("b", 99)], [7; 32]);
        assert_eq!(
            manifest().disagreements(&changed),
            vec![ManifestField::Packs]
        );
    }

    #[test]
    fn a_field_refuses_the_rungs_that_rest_on_it_and_no_others() {
        let held = manifest();
        let refused = |edit: fn(&mut ArtefactIdentityManifest)| {
            let mut artefact = manifest();
            edit(&mut artefact);
            artefact.refused_rungs(&held)
        };
        let packs = RungSet::of(Rung::PackFacts).with(Rung::BuiltinAlias);
        assert_eq!(refused(|m| m.packs.push(stamp("c", 3))), packs);
        assert!(!refused(|m| m.packs.push(stamp("c", 3))).contains(Rung::Generic));
        assert_eq!(refused(|m| m.packages.clear()), packs);
        assert_eq!(
            refused(|m| m.intrinsic_table_hash[0] ^= 1),
            RungSet::of(Rung::ShippedBacking)
        );
        assert_eq!(
            refused(|m| m.embedded_stdlib_revision.clear()),
            RungSet::of(Rung::ShippedBacking)
        );
        for edit in [
            (|m: &mut ArtefactIdentityManifest| m.abi_version += 1) as fn(&mut _),
            |m| m.environment.clear(),
            |m| m.release.clear(),
            |m| m.build = BuildProfileId::Unknown,
        ] {
            assert_eq!(refused(edit), RungSet::ALL);
        }
        assert!(refused(|_| {}).is_empty());
    }

    #[test]
    fn a_field_is_described_as_a_message_spells_it() {
        let mut manifest = manifest();
        manifest.abi_version = 7;
        manifest.environment = "tcl8.6".to_owned();
        manifest.release = "8.6".to_owned();
        manifest.build = BuildProfileId::JimFull;
        manifest.packages = vec![
            ("Tk".to_owned(), "8.6".to_owned()),
            ("vendor".to_owned(), "2.1".to_owned()),
        ];
        manifest.packs = vec![stamp("vendor", 0xab), stamp("other", 0x1f)];
        manifest.intrinsic_table_hash = [0xa5; 32];
        manifest.embedded_stdlib_revision = "9.0.4+abc.def".to_owned();
        for (field, expected) in [
            (ManifestField::AbiVersion, "7".to_owned()),
            (ManifestField::Environment, "tcl8.6".to_owned()),
            (ManifestField::Release, "8.6".to_owned()),
            (ManifestField::Build, "JimFull".to_owned()),
            (ManifestField::Packages, "Tk 8.6, vendor 2.1".to_owned()),
            (ManifestField::Packs, "vendor@ab, other@1f".to_owned()),
            (ManifestField::IntrinsicTableHash, "a5".repeat(32)),
            (
                ManifestField::EmbeddedStdlibRevision,
                "9.0.4+abc.def".to_owned(),
            ),
        ] {
            assert_eq!(manifest.describe(field), expected, "{}", field.name());
        }
    }

    #[test]
    fn a_rung_set_holds_what_was_put_in_it() {
        let set = RungSet::of(Rung::PackFacts).with(Rung::ShippedBacking);
        assert!(set.contains(Rung::PackFacts) && set.contains(Rung::ShippedBacking));
        assert!(!set.contains(Rung::Generic) && !set.contains(Rung::BuiltinAlias));
        assert!(set.intersects(RungSet::of(Rung::ShippedBacking)));
        assert!(!set.intersects(RungSet::of(Rung::ReferenceBody)));
        assert!(RungSet::EMPTY.is_empty() && !set.is_empty());
        for rung in [
            Rung::Generic,
            Rung::PackFacts,
            Rung::BuiltinAlias,
            Rung::ReferenceBody,
            Rung::ShippedBacking,
        ] {
            assert!(RungSet::ALL.contains(rung));
        }
    }

    fn module_with(section: Option<&[u8]>) -> Vec<u8> {
        module_with_other_section_first(section, false)
    }

    fn module_with_other_section_first(section: Option<&[u8]>, other_first: bool) -> Vec<u8> {
        fn leb(mut value: usize) -> Vec<u8> {
            let mut out = Vec::new();
            loop {
                let byte = u8::try_from(value & 0x7f).unwrap();
                value >>= 7;
                if value == 0 {
                    out.push(byte);
                    return out;
                }
                out.push(byte | 0x80);
            }
        }
        let mut module = b"\0asm\x01\0\0\0".to_vec();
        // a type section with no types, so the walk has something to skip
        module.extend([1, 1, 0]);
        if other_first {
            // a custom section of another name, which is not the manifest
            module.extend([0, 8, 4, b'n', b'a', b'm', b'e', 1, 2, 3]);
        }
        if let Some(payload) = section {
            let mut content = leb(WASM_SECTION.len());
            content.extend(WASM_SECTION.as_bytes());
            content.extend(payload);
            module.push(0);
            module.extend(leb(content.len()));
            module.extend(content);
        }
        module.extend([11, 1, 0]);
        module
    }

    #[test]
    fn a_module_hands_back_the_manifest_in_its_custom_section() {
        let manifest = manifest();
        let module = module_with(Some(&manifest.to_bytes()));
        assert_eq!(
            ArtefactIdentityManifest::from_wasm(&module),
            Ok(Some(manifest.clone()))
        );
        assert_eq!(
            ArtefactIdentityManifest::from_wasm(&module_with(None)),
            Ok(None)
        );
        // Another custom section is neither the manifest nor in its way.
        assert_eq!(
            ArtefactIdentityManifest::from_wasm(&module_with_other_section_first(None, true)),
            Ok(None)
        );
        assert_eq!(
            ArtefactIdentityManifest::from_wasm(&module_with_other_section_first(
                Some(&manifest.to_bytes()),
                true
            )),
            Ok(Some(manifest.clone()))
        );
        assert_eq!(
            ArtefactIdentityManifest::from_wasm(b"not a module"),
            Err(ManifestDecodeError::NotAModule)
        );
        assert!(ArtefactIdentityManifest::from_wasm(&module_with(Some(&[1, 2, 3]))).is_err());
    }

    #[test]
    fn the_embedded_revision_is_a_patchlevel_a_commit_and_a_digest() {
        let (patchlevel, rest) = EMBEDDED_STDLIB_REVISION
            .split_once('+')
            .expect("a patchlevel and what follows");
        assert_eq!(patchlevel.split('.').count(), 3);
        assert!(
            patchlevel
                .split('.')
                .all(|part| part.parse::<u32>().is_ok())
        );
        let (commit, digest) = rest.split_once('.').expect("a commit and a digest");
        for hex in [commit, digest] {
            assert_eq!(hex.len(), 12);
            assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }
}
