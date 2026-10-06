// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected native scalar getters, independently of expression numeral grammar.
//!
//! The caller retains the original object and applies the returned cache change
//! even when the getter fails. These pure recipes supply no object identity,
//! interpreter-effect closure, expression-source or compiler permission.

use std::borrow::Cow;

use tcl_dialect::{
    TclVersion,
    model::{BuildProfileId, DialectPoint, Family, Release},
};

use crate::number::{self, Number, ParseFlags, Radix};

mod errors;
mod float;
#[path = "scalar_getter/number.rs"]
mod number_getter;
pub use errors::{NativeScalarGetterError, NativeScalarGetterErrorCode};
pub use number_getter::{NativeNumberGetterConversion, NativeNumberGetterKind};

/// C8.4 compiler's counted `TclLooksLikeInt`/`TclParseInteger` prefix predicate.
/// This is not full integer acceptance: the selected fresh `GetInt` conversion
/// must independently validate the complete spelling and native width.
#[must_use]
pub fn compiler_integer_prefix84(mut bytes: &[u8]) -> bool {
    while bytes.first().is_some_and(u8::is_ascii_whitespace) {
        bytes = &bytes[1..];
    }
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        bytes = &bytes[1..];
    }
    if bytes.len() > 1 && bytes[0] == b'0' && matches!(bytes[1], b'x' | b'X') {
        return true;
    }
    let digits = bytes
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    digits > 0 && !matches!(bytes.get(digits), Some(b'.' | b'e' | b'E'))
}

/// Original-object legacy increment transaction, independent of numeric grammar.
/// Each recipe describes the audited native signed 64-bit endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeLegacyIncrementRecipe {
    /// C8.4 converts the amount before lookup and copies current before probing.
    Tcl84,
    /// Jim converts safe-expression amount before lookup and probes before COW.
    Jim084,
}

/// The reached primitive getter, distinct from arithmetic numeric interpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeScalarGetterKind {
    /// C Tcl primitive signed 32-bit extraction, distinct from Wide and Jim Long.
    Int,
    /// Pinned C Tcl 8.4 native-long extraction, preserving its long cache.
    Long,
    /// Native wide-integer extraction, including its release-specific width.
    Wide,
    /// Explicit native double extraction and its own cache conversion.
    Double,
    /// Native boolean extraction, distinct from mathematical numeric truth.
    Boolean,
}

/// Actual original storage before native string materialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeScalarStringStorage {
    /// Exact raw string bytes; no byte-array encoding is implied.
    RawString,
    /// Original C Tcl byte-array bytes, before their modified UTF-8 projection.
    ByteArray,
}

/// Actual numeric or word-boolean cache, independently of getter return value.
#[derive(Debug, Clone, PartialEq)]
pub enum NativeScalarCache {
    /// C Tcl 8.4 native-long primary cache, distinct from its wideInt type.
    Tcl84Long(i64),
    /// The full parsed magnitude/category; a wrapped Wide result may differ.
    Number(Number),
    /// C's non-integer boolean-word representation.
    WordBoolean(bool),
    /// Jim's exact retained integer under its explicit double cache.
    JimCoercedInteger(i64),
}

/// Successful value returned by a native getter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NativeScalarGetterValue {
    /// Returned native wide integer; not necessarily the cached magnitude.
    Wide(i64),
    /// Returned native floating-point value.
    Double(f64),
    /// Boolean interpretation; the original numeric cache may remain unchanged.
    Boolean(bool),
}

/// Native Jim expression tree construction's numeric term result.
/// This constructs a term rather than converting an existing operand cache.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JimExpressionNumber {
    /// Exact wrapped native integer constructor argument.
    Integer(i64),
    /// Native floating-point constructor argument, including infinities.
    Double(f64),
}

/// Parse a complete Jim expression numeric token for fresh term construction.
/// Numeric cache conversion, errno-sensitive getters and diagnostic state are
/// separate operations. NUL or incomplete spellings remain String terms.
#[must_use]
pub fn jim_expression_number(token: &[u8]) -> Option<JimExpressionNumber> {
    let scanned =
        tcl_dialect::scan_jim_expression_number(token, 0, tcl_dialect::NumberSyntax::Jim)?;
    if scanned.end() != token.len() {
        return None;
    }
    jim_expression_number_for_kind(token, scanned.jim_kind()?)
}

/// Construct a fresh term using the original Jim scanner's selected token kind.
/// A failed complete conversion leaves the original counted String term.
#[must_use]
pub fn jim_expression_number_for_kind(
    token: &[u8],
    kind: tcl_dialect::JimExpressionNumberKind,
) -> Option<JimExpressionNumber> {
    if token.contains(&0) || token.iter().any(u8::is_ascii_whitespace) {
        return None;
    }
    match kind {
        tcl_dialect::JimExpressionNumberKind::Integer => {
            float::jim_unsigned_integer(token, None).map(JimExpressionNumber::Integer)
        }
        tcl_dialect::JimExpressionNumberKind::Double => {
            float::parse_c_double(token).map(|parsed| JimExpressionNumber::Double(parsed.value))
        }
    }
}

/// Native guest failure after an independently selected getter was reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeScalarGetterFailure {
    /// The materialized spelling or current cache cannot satisfy this getter.
    Invalid,
    /// Native numeric parser retained its invalid implicit-octal state.
    InvalidOctal,
    /// Wide extraction rejected an already cached floating-point number.
    /// This bypasses fresh spelling diagnostics and has a distinct code update.
    CachedNonInteger,
    /// Native wide extraction rejected the magnitude, after any numeric cache.
    IntegerOverflow,
    /// C `GetInt` reached its narrower signed/unsigned native int width check.
    IntWidthOverflow,
    /// Native C double/boolean extraction rejected NaN after caching it.
    FloatingPointNaN,
    /// C strtod reported its independently observed domain error.
    FloatingPointDomain,
    /// C strtod failed with an otherwise unclassified actual errno.
    FloatingPointUnknown(i32),
    /// Legacy C `strtod` set its range error; native presentation distinguishes
    /// a zero return from every other returned value, including subnormals.
    FloatingPointRange {
        /// Whether native conversion returned zero (the underflow presentation).
        result_is_zero: bool,
    },
}

/// A native cache transition plus the independent getter completion.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeScalarGetterConversion {
    cache: Option<NativeScalarCache>,
    outcome: Result<NativeScalarGetterValue, NativeScalarGetterFailure>,
    requires_string_materialization: bool,
}

impl NativeScalarGetterConversion {
    /// Cache change on the original object, including changes before an error.
    #[must_use]
    pub fn cache(&self) -> Option<&NativeScalarCache> {
        self.cache.as_ref()
    }

    /// Whether the original object's string must be retained before applying
    /// this cache transition, even when its numeric value was already cached.
    #[must_use]
    pub const fn requires_string_materialization(&self) -> bool {
        self.requires_string_materialization
    }

    /// Getter return or guest failure, independently of its prior cache change.
    pub const fn outcome(&self) -> Result<NativeScalarGetterValue, NativeScalarGetterFailure> {
        self.outcome
    }

    /// Consume the preparation obligation, cache transition and completion.
    /// The adapter must complete required string materialization before changing
    /// the cache, and apply that change before publishing either completion.
    pub fn into_parts(
        self,
    ) -> (
        bool,
        Option<NativeScalarCache>,
        Result<NativeScalarGetterValue, NativeScalarGetterFailure>,
    ) {
        (
            self.requires_string_materialization,
            self.cache,
            self.outcome,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Engine {
    Tcl(TclVersion),
    Jim084,
}

/// Audited actual native getter recipe; construction does not attest an object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeScalarGetterProtocol {
    engine: Engine,
}

#[derive(Debug)]
enum CachedConversionFrontier {
    Continue,
    Resolved(Option<NativeScalarGetterConversion>),
}

/// Selected first `strtoull` call of `JimNumberBase`/`jim_strtoull`. It supplies
/// syntax geometry only; the actual host owns the conversion and errno.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeJimUnsignedStage {
    /// Original `CString` byte offset after an admitted explicit prefix.
    pub offset: usize,
    /// Native radix for the reached C call.
    pub base: u32,
    /// Apply the separately stripped negative sign to the unsigned payload.
    pub negate: bool,
}

impl NativeScalarGetterProtocol {
    /// Adopt a fresh C8.4 strtod call, including its complete host errno facts.
    /// Existing cached Double conversions must not reach this constructor.
    #[must_use]
    pub fn c84_double_from_host(
        self,
        original: &[u8],
        value: f64,
        end: usize,
        errno: i32,
        domain_error: bool,
        range_error: bool,
    ) -> Option<NativeScalarGetterConversion> {
        if self.tcl_version() != Some(TclVersion::V8_4) {
            return None;
        }
        if end == 0 {
            return Some(self.invalid_conversion(NativeScalarGetterKind::Double, original));
        }
        if errno != 0 {
            let failure = match crate::expr::errors::NativeFloatError::classify(
                value,
                errno,
                domain_error,
                range_error,
            ) {
                crate::expr::errors::NativeFloatError::Domain => {
                    NativeScalarGetterFailure::FloatingPointDomain
                }
                crate::expr::errors::NativeFloatError::Underflow => {
                    NativeScalarGetterFailure::FloatingPointRange {
                        result_is_zero: true,
                    }
                }
                crate::expr::errors::NativeFloatError::Overflow => {
                    NativeScalarGetterFailure::FloatingPointRange {
                        result_is_zero: false,
                    }
                }
                crate::expr::errors::NativeFloatError::Unknown(errno) => {
                    NativeScalarGetterFailure::FloatingPointUnknown(errno)
                }
            };
            return Some(convert(None, Err(failure)));
        }
        if end > original.len() || !original[end..].iter().all(u8::is_ascii_whitespace) {
            return Some(self.invalid_conversion(NativeScalarGetterKind::Double, original));
        }
        Some(convert(
            Some(NativeScalarCache::Number(Number::Double(value))),
            Ok(NativeScalarGetterValue::Double(value)),
        ))
    }

    /// Adopt C8.4's reached integer C-call end pointer and range fact.
    /// Native range failure precedes trailing-character validation.
    #[must_use]
    pub fn c84_integer_from_host(
        self,
        kind: NativeScalarGetterKind,
        original: &[u8],
        start: usize,
        end: usize,
        range_error: bool,
    ) -> Option<NativeScalarGetterConversion> {
        if self.tcl_version() != Some(TclVersion::V8_4)
            || !matches!(
                kind,
                NativeScalarGetterKind::Int
                    | NativeScalarGetterKind::Long
                    | NativeScalarGetterKind::Wide
            )
        {
            return None;
        }
        if end <= start || end > original.len() {
            return Some(self.invalid_conversion(kind, original));
        }
        if range_error {
            return Some(convert(
                None,
                Err(NativeScalarGetterFailure::IntegerOverflow),
            ));
        }
        if !original[end..].iter().all(u8::is_ascii_whitespace) {
            return Some(self.invalid_conversion(kind, original));
        }
        self.fresh_conversion(kind, original)
    }

    /// Select Jim's first integer host call without parsing an object or
    /// assuming numeric thread state. A prefix call with no digits falls back
    /// to the whole original decimal input in the concrete host adapter.
    #[must_use]
    pub fn jim_unsigned_stage(self, materialized: &[u8]) -> Option<NativeJimUnsignedStage> {
        self.is_jim084()
            .then(|| float::jim_unsigned_stage(nul_prefix(materialized)))
    }

    /// `JimCheckConversion`'s exact original/end-pointer validation.
    #[must_use]
    pub fn jim_host_conversion_complete(self, materialized: &[u8], end: usize) -> bool {
        let input = nul_prefix(materialized);
        self.is_jim084()
            && !input.is_empty()
            && end > 0
            && end <= input.len()
            && input[end..]
                .iter()
                .all(|&byte| byte == b' ' || (b'\t'..=b'\r').contains(&byte))
    }

    /// Adopt the independently executed Jim integer C conversion stage.
    /// Host facts grant no authority for a foreign native getter protocol.
    #[must_use]
    pub fn jim_wide_from_host(
        self,
        materialized: &[u8],
        value: i64,
        end: usize,
        range_error: bool,
    ) -> Option<NativeScalarGetterConversion> {
        if !self.is_jim084() {
            return None;
        }
        if !self.jim_host_conversion_complete(materialized, end) {
            return Some(invalid());
        }
        Some(if range_error && matches!(value, i64::MIN | i64::MAX) {
            convert(None, Err(NativeScalarGetterFailure::IntegerOverflow))
        } else {
            convert(
                Some(NativeScalarCache::Number(Number::Int(value))),
                Ok(NativeScalarGetterValue::Wide(value)),
            )
        })
    }

    /// Adopt a reached Jim `GetDouble` stage. Decimal-integer success installs
    /// its coerced-integer primary; only the later strtod stage installs Double.
    #[must_use]
    pub fn jim_double_from_host(
        self,
        materialized: &[u8],
        end: usize,
        value: f64,
        integer: Option<i64>,
    ) -> Option<NativeScalarGetterConversion> {
        if !self.is_jim084() {
            return None;
        }
        if !self.jim_host_conversion_complete(materialized, end) {
            return Some(invalid());
        }
        Some(match integer {
            Some(integer) => convert(
                Some(NativeScalarCache::JimCoercedInteger(integer)),
                Ok(NativeScalarGetterValue::Double(float::integer_double(
                    integer,
                ))),
            ),
            None => convert(
                Some(NativeScalarCache::Number(Number::Double(value))),
                Ok(NativeScalarGetterValue::Double(value)),
            ),
        })
    }
    /// Recipe for an independently authenticated native C Tcl release.
    #[must_use]
    pub const fn for_tcl_version(version: TclVersion) -> Self {
        Self {
            engine: Engine::Tcl(version),
        }
    }

    /// Select an audited engine/build point; foreign and unknown builds abstain.
    #[must_use]
    pub fn for_point(point: DialectPoint) -> Option<Self> {
        match (point.family(), point.build()) {
            (Family::Tcl, BuildProfileId::Canonical) => {
                point.tcl_version().map(Self::for_tcl_version)
            }
            (Family::Jim, BuildProfileId::Canonical | BuildProfileId::JimFull)
                if point.release() == Release::JIM_0_84 =>
            {
                Some(Self {
                    engine: Engine::Jim084,
                })
            }
            _ => None,
        }
    }

    /// Whether this protocol is the audited pinned Jim getter implementation.
    #[must_use]
    pub const fn is_jim084(self) -> bool {
        matches!(self.engine, Engine::Jim084)
    }

    /// Actual C release, without borrowing a vendor compatibility release.
    #[must_use]
    pub const fn tcl_version(self) -> Option<TclVersion> {
        match self.engine {
            Engine::Tcl(version) => Some(version),
            Engine::Jim084 => None,
        }
    }

    /// Reached C8.4 expression `GET_WIDE_OR_INT` conversion. Existing integer
    /// primaries are read without changing their long/wide distinction; fresh
    /// accepted integers use the audited native-long cache. This is separate
    /// from the primitive Wide getter, which installs a `wideInt` primary.
    #[must_use]
    pub fn expression_integer_conversion84(
        self,
        current: Option<&NativeScalarCache>,
        materialized: &[u8],
    ) -> Option<NativeScalarGetterConversion> {
        if self.tcl_version() != Some(TclVersion::V8_4) {
            return None;
        }
        match current {
            Some(
                NativeScalarCache::Tcl84Long(value) | NativeScalarCache::Number(Number::Int(value)),
            ) => Some(convert(None, Ok(NativeScalarGetterValue::Wide(*value)))),
            Some(NativeScalarCache::Number(Number::Double(_) | Number::Nan { .. }))
                if !self.expression_integer_spelling84(materialized) =>
            {
                None
            }
            _ if self.expression_integer_spelling84(materialized) => {
                Some(self.fresh_long84(materialized))
            }
            _ => None,
        }
    }

    /// C8.4 `TclLooksLikeInt` selects the reached expression conversion from
    /// the original spelling's prefix, independently of range or full parse
    /// success. The caller supplies resident bytes for a cached Double.
    #[must_use]
    pub fn expression_integer_spelling84(self, materialized: &[u8]) -> bool {
        if self.tcl_version() != Some(TclVersion::V8_4) {
            return false;
        }
        let mut input = materialized;
        while input.first().is_some_and(u8::is_ascii_whitespace) {
            input = &input[1..];
        }
        if matches!(input.first(), Some(b'+' | b'-')) {
            input = &input[1..];
        }
        if input.starts_with(b"0x") || input.starts_with(b"0X") {
            return true;
        }
        let digits = input
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        digits != 0 && !matches!(input.get(digits), Some(b'.' | b'e' | b'E'))
    }

    /// C8.4 integer arithmetic keeps `wideInt` results when a reached operand
    /// is `wideInt`; otherwise `Tcl_NewLongObj` produces the native-long primary.
    #[must_use]
    pub fn expression_integer_result84(
        self,
        integer: i64,
        operands: &[Option<NativeScalarCache>],
    ) -> Option<NativeScalarCache> {
        (self.tcl_version() == Some(TclVersion::V8_4)).then(|| {
            if operands
                .iter()
                .any(|cache| matches!(cache, Some(NativeScalarCache::Number(Number::Int(_)))))
            {
                NativeScalarCache::Number(Number::Int(integer))
            } else {
                NativeScalarCache::Tcl84Long(integer)
            }
        })
    }

    /// Produce the actual getter string bytes from independently proved storage.
    /// `None` means that storage/engine combination is not modeled, not an error.
    #[must_use]
    pub fn materialize(
        self,
        storage: NativeScalarStringStorage,
        original: &[u8],
    ) -> Option<Cow<'_, [u8]>> {
        use crate::native_string::{NativeStringInput, NativeStringProtocol};
        let protocol = match self.engine {
            Engine::Tcl(version) => NativeStringProtocol::C(version),
            Engine::Jim084 => NativeStringProtocol::Jim084,
        };
        let input = match storage {
            NativeScalarStringStorage::RawString => NativeStringInput::ResidentString(original),
            NativeScalarStringStorage::ByteArray => NativeStringInput::PureByteArray(original),
        };
        protocol.materialize(input).ok()
    }

    /// Current numeric-cache fast path. `None` requires actual string access;
    /// callers must not manufacture that string from a rounded getter return.
    #[must_use]
    pub fn cached_conversion(
        self,
        kind: NativeScalarGetterKind,
        cache: &NativeScalarCache,
    ) -> Option<NativeScalarGetterConversion> {
        use NativeScalarCache::{JimCoercedInteger, Number as CachedNumber, WordBoolean};
        use NativeScalarGetterKind::{Boolean, Double, Wide};
        if let CachedConversionFrontier::Resolved(conversion) =
            self.c_integer_cache_frontier(kind, cache)
        {
            return conversion;
        }
        match (self.engine, kind, cache) {
            (Engine::Jim084, Wide, JimCoercedInteger(value)) => Some(convert(
                Some(CachedNumber(Number::Int(*value))),
                Ok(NativeScalarGetterValue::Wide(*value)),
            )),
            (Engine::Jim084, Double, JimCoercedInteger(value)) => Some(convert(
                None,
                Ok(NativeScalarGetterValue::Double(float::integer_double(
                    *value,
                ))),
            )),
            (Engine::Jim084, Double, CachedNumber(Number::Int(value)))
                if (-(1_i64 << 53)..=(1_i64 << 53) - 1).contains(value) =>
            {
                Some(convert(
                    Some(JimCoercedInteger(*value)),
                    Ok(NativeScalarGetterValue::Double(float::integer_double(
                        *value,
                    ))),
                ))
            }
            (Engine::Jim084, _, WordBoolean(_))
            | (Engine::Tcl(_), _, JimCoercedInteger(_))
            | (
                Engine::Tcl(TclVersion::V8_4),
                Double,
                CachedNumber(Number::Int(_) | Number::Big { .. }),
            )
            | (Engine::Jim084, Double, CachedNumber(Number::Int(_) | Number::Big { .. })) => None,
            (_, Wide, CachedNumber(Number::Int(value))) => {
                Some(convert(None, Ok(NativeScalarGetterValue::Wide(*value))))
            }
            (Engine::Tcl(version), Wide, CachedNumber(Number::Big { .. }))
                if version != TclVersion::V8_4 =>
            {
                Some(convert(None, self.wide_value(cache_number(cache)?)))
            }
            (Engine::Tcl(version), Wide, CachedNumber(Number::Double(_) | Number::Nan { .. }))
                if version != TclVersion::V8_4 =>
            {
                Some(convert(
                    None,
                    Err(NativeScalarGetterFailure::CachedNonInteger),
                ))
            }
            (_, Double, CachedNumber(number)) => Some(convert(None, self.double_value(number))),
            (_, Boolean, WordBoolean(value)) => {
                Some(convert(None, Ok(NativeScalarGetterValue::Boolean(*value))))
            }
            (Engine::Jim084, Boolean, CachedNumber(Number::Int(value))) => Some(convert(
                None,
                Ok(NativeScalarGetterValue::Boolean(*value != 0)),
            )),
            (Engine::Tcl(TclVersion::V8_4), Boolean, CachedNumber(number)) => {
                let value = number_truth(number);
                let mut conversion = convert(
                    Some(WordBoolean(value)),
                    Ok(NativeScalarGetterValue::Boolean(value)),
                );
                conversion.requires_string_materialization = true;
                Some(conversion)
            }
            (Engine::Tcl(_), Boolean, CachedNumber(number)) => {
                Some(convert(None, self.boolean_value(number)))
            }
            _ => None,
        }
    }

    // C's native-long cache and GetInt narrowing precede the shared numeric fast path.
    // A settled decline requires original string access, rather than another cache arm.
    fn c_integer_cache_frontier(
        self,
        kind: NativeScalarGetterKind,
        cache: &NativeScalarCache,
    ) -> CachedConversionFrontier {
        use NativeScalarGetterKind::Long;
        if kind == Long && self.tcl_version() != Some(TclVersion::V8_4) {
            return CachedConversionFrontier::Resolved(None);
        }
        if let NativeScalarCache::Tcl84Long(value) = cache {
            if self.tcl_version() != Some(TclVersion::V8_4) {
                return CachedConversionFrontier::Resolved(None);
            }
            return CachedConversionFrontier::Resolved(match kind {
                NativeScalarGetterKind::Wide => Some(convert(
                    Some(NativeScalarCache::Number(Number::Int(*value))),
                    Ok(NativeScalarGetterValue::Wide(*value)),
                )),
                Long => Some(convert(None, Ok(NativeScalarGetterValue::Wide(*value)))),
                NativeScalarGetterKind::Int => {
                    Some(self.narrow_int(convert(None, Ok(NativeScalarGetterValue::Wide(*value)))))
                }
                NativeScalarGetterKind::Double => None,
                NativeScalarGetterKind::Boolean => {
                    let mut conversion = convert(
                        Some(NativeScalarCache::WordBoolean(*value != 0)),
                        Ok(NativeScalarGetterValue::Boolean(*value != 0)),
                    );
                    conversion.requires_string_materialization = true;
                    Some(conversion)
                }
            });
        }
        if kind == Long {
            return CachedConversionFrontier::Resolved(match cache {
                NativeScalarCache::Number(Number::Int(value)) => {
                    Some(convert(None, Ok(NativeScalarGetterValue::Wide(*value))))
                }
                _ => None,
            });
        }
        if kind == NativeScalarGetterKind::Int {
            if self.is_jim084() {
                return CachedConversionFrontier::Resolved(None);
            }
            let conversion = if self.tcl_version() == Some(TclVersion::V8_6) {
                match cache {
                    NativeScalarCache::Number(number)
                        if matches!(number, Number::Big { .. } | Number::Nan { .. })
                            || matches!(number, Number::Double(value) if value.is_nan()) =>
                    {
                        Some(convert(
                            None,
                            Err(NativeScalarGetterFailure::IntWidthOverflow),
                        ))
                    }
                    _ => self.cached_conversion(NativeScalarGetterKind::Wide, cache),
                }
            } else {
                self.cached_conversion(NativeScalarGetterKind::Wide, cache)
            };
            return CachedConversionFrontier::Resolved(
                conversion.map(|conversion| self.narrow_int(conversion)),
            );
        }
        CachedConversionFrontier::Continue
    }

    /// Parse materialized bytes without assuming a hidden native range state.
    /// Jim's signed boundary Wide values depend on retained native `errno`;
    /// those inputs abstain until the caller supplies that independent state.
    #[must_use]
    pub fn fresh_conversion(
        self,
        kind: NativeScalarGetterKind,
        materialized: &[u8],
    ) -> Option<NativeScalarGetterConversion> {
        if (kind == NativeScalarGetterKind::Long && self.tcl_version() != Some(TclVersion::V8_4))
            || (self.is_jim084() && kind == NativeScalarGetterKind::Int)
        {
            return None;
        }
        if self.is_jim084()
            && kind == NativeScalarGetterKind::Wide
            && float::jim_unsigned_integer(nul_prefix(materialized), None)
                .is_some_and(|value| value == i64::MIN || value == i64::MAX)
        {
            return None;
        }
        Some(self.fresh_conversion_with_range_error(kind, materialized, false))
    }

    /// Parse with an independently retained native `errno == ERANGE` fact.
    /// This is a native environment input, never inferred from object bytes or
    /// the last guest error. The caller still owns all native errno updates.
    #[must_use]
    pub fn fresh_conversion_with_range_error(
        self,
        kind: NativeScalarGetterKind,
        materialized: &[u8],
        prior_range_error: bool,
    ) -> NativeScalarGetterConversion {
        match kind {
            NativeScalarGetterKind::Int => self.fresh_int(materialized),
            NativeScalarGetterKind::Long => self.fresh_long84(materialized),
            NativeScalarGetterKind::Wide => self.fresh_wide(materialized, prior_range_error),
            NativeScalarGetterKind::Double => self.fresh_double(materialized),
            NativeScalarGetterKind::Boolean => self.fresh_boolean(materialized),
        }
    }

    /// Probe Jim's decimal `Jim_StringToWide` spelling without an object cache
    /// transition or primitive error presentation. This is distinct from
    /// `GetWide`'s base-zero conversion and return-code object conversion.
    /// `JimCheckConversion` validates the end pointer independently of errno;
    /// this string-only stage does not inherit `GetWide`'s range-state dependency.
    /// `None` means the selected Jim protocol is unavailable.
    #[must_use]
    pub fn jim_decimal_wide_probe(
        self,
        materialized: &[u8],
    ) -> Option<Result<i64, NativeScalarGetterFailure>> {
        if !self.is_jim084() {
            return None;
        }
        let Some(value) = float::jim_unsigned_integer(nul_prefix(materialized), Some(10)) else {
            return Some(Err(NativeScalarGetterFailure::Invalid));
        };
        Some(Ok(value))
    }

    /// Probe the pinned Jim absolute-frame suffix's signed `jim_strtol`
    /// stage. It consumes a `CString` suffix, without an object cache or errno
    /// dependency. `None` declines foreign engines; an inner error is a native
    /// spelling failure, independently of frame existence.
    #[must_use]
    pub fn jim_absolute_frame_probe(
        self,
        original: &[u8],
    ) -> Option<Result<i64, NativeScalarGetterFailure>> {
        self.is_jim084().then(|| {
            float::jim_frame_long(nul_prefix(original)).ok_or(NativeScalarGetterFailure::Invalid)
        })
    }

    fn parser_input(self, materialized: &[u8]) -> &[u8] {
        if self.engine == Engine::Tcl(TclVersion::V8_4) {
            materialized
        } else {
            nul_prefix(materialized)
        }
    }

    fn fresh_long84(self, materialized: &[u8]) -> NativeScalarGetterConversion {
        let mut conversion = self.fresh_wide(materialized, false);
        if let Some(NativeScalarCache::Number(Number::Int(value))) = conversion.cache {
            conversion.cache = Some(NativeScalarCache::Tcl84Long(value));
        }
        conversion
    }

    fn fresh_int(self, materialized: &[u8]) -> NativeScalarGetterConversion {
        if self.tcl_version() == Some(TclVersion::V8_4) {
            return self.narrow_int(self.fresh_long84(materialized));
        }
        if self.tcl_version() == Some(TclVersion::V8_6) {
            let input = self.parser_input(materialized);
            let Some(number) = parse_number(input, TclVersion::V8_6, false) else {
                return invalid();
            };
            let outcome = match &number {
                Number::Int(value) => Ok(NativeScalarGetterValue::Wide(*value)),
                Number::Double(_) => Err(NativeScalarGetterFailure::CachedNonInteger),
                Number::Big { .. } | Number::Nan { .. } => {
                    Err(NativeScalarGetterFailure::IntWidthOverflow)
                }
            };
            return self.narrow_int(convert(Some(NativeScalarCache::Number(number)), outcome));
        }
        self.narrow_int(self.fresh_wide(materialized, false))
    }

    fn narrow_int(
        self,
        mut conversion: NativeScalarGetterConversion,
    ) -> NativeScalarGetterConversion {
        if let Ok(NativeScalarGetterValue::Wide(value)) = conversion.outcome {
            let lower = if self
                .tcl_version()
                .is_some_and(|version| version >= TclVersion::V9_0)
            {
                i64::from(i32::MIN)
            } else {
                -i64::from(u32::MAX)
            };
            conversion.outcome = if value < lower || value > i64::from(u32::MAX) {
                Err(NativeScalarGetterFailure::IntWidthOverflow)
            } else {
                Ok(NativeScalarGetterValue::Wide(i64::from(
                    crate::number::native_int32_low_bits(value),
                )))
            };
        }
        conversion
    }

    fn fresh_wide(
        self,
        materialized: &[u8],
        prior_range_error: bool,
    ) -> NativeScalarGetterConversion {
        let input = self.parser_input(materialized);
        if self.is_jim084() {
            return match float::jim_unsigned_integer(input, None) {
                Some(value) if prior_range_error && (value == i64::MIN || value == i64::MAX) => {
                    convert(None, Err(NativeScalarGetterFailure::IntegerOverflow))
                }
                Some(value) => convert(
                    Some(NativeScalarCache::Number(Number::Int(value))),
                    Ok(NativeScalarGetterValue::Wide(value)),
                ),
                None => invalid(),
            };
        }
        let Engine::Tcl(version) = self.engine else {
            unreachable!()
        };
        let Some(number) = parse_number(input, version, true) else {
            return self.invalid_conversion(NativeScalarGetterKind::Wide, input);
        };
        let outcome = self.wide_value(&number);
        // C8.4 SetWideInt stores the returned wide; later C retains its bignum.
        let cache = if version == TclVersion::V8_4 {
            match outcome {
                Ok(NativeScalarGetterValue::Wide(value)) => {
                    Some(NativeScalarCache::Number(Number::Int(value)))
                }
                _ => None,
            }
        } else {
            Some(NativeScalarCache::Number(number))
        };
        convert(cache, outcome)
    }

    fn fresh_double(self, materialized: &[u8]) -> NativeScalarGetterConversion {
        let input = self.parser_input(materialized);
        if self.is_jim084()
            && let Some(value) = float::jim_unsigned_integer(input, Some(10))
        {
            return convert(
                Some(NativeScalarCache::JimCoercedInteger(value)),
                Ok(NativeScalarGetterValue::Double(float::integer_double(
                    value,
                ))),
            );
        }
        if self.is_jim084() || self.engine == Engine::Tcl(TclVersion::V8_4) {
            let Some(parsed) = float::parse_c_double(input) else {
                return self.invalid_conversion(NativeScalarGetterKind::Double, input);
            };
            if !self.is_jim084() && parsed.range_error {
                return convert(
                    None,
                    Err(NativeScalarGetterFailure::FloatingPointRange {
                        result_is_zero: parsed.value == 0.0,
                    }),
                );
            }
            return convert(
                Some(NativeScalarCache::Number(Number::Double(parsed.value))),
                Ok(NativeScalarGetterValue::Double(parsed.value)),
            );
        }
        let Engine::Tcl(version) = self.engine else {
            unreachable!()
        };
        let Some(number) = parse_number(input, version, false) else {
            return self.invalid_conversion(NativeScalarGetterKind::Double, input);
        };
        let outcome = self.double_value(&number);
        convert(Some(NativeScalarCache::Number(number)), outcome)
    }

    fn fresh_boolean(self, materialized: &[u8]) -> NativeScalarGetterConversion {
        if self.is_jim084() {
            let value = match nul_prefix(materialized) {
                b"1" | b"true" | b"yes" | b"on" => true,
                b"0" | b"false" | b"no" | b"off" => false,
                _ => return invalid(),
            };
            return convert(
                Some(NativeScalarCache::Number(Number::Int(i64::from(value)))),
                Ok(NativeScalarGetterValue::Boolean(value)),
            );
        }
        if self.engine == Engine::Tcl(TclVersion::V8_4) {
            return Self::fresh_boolean84(materialized);
        }
        if let Ok(text) = std::str::from_utf8(materialized)
            && let Some(value) = crate::boolean::parse_boolean_strict(text)
        {
            let cache = if materialized == b"0" || materialized == b"1" {
                NativeScalarCache::Number(Number::Int(i64::from(value)))
            } else {
                NativeScalarCache::WordBoolean(value)
            };
            return convert(Some(cache), Ok(NativeScalarGetterValue::Boolean(value)));
        }
        let Engine::Tcl(version) = self.engine else {
            unreachable!()
        };
        let Some(number) = parse_number(nul_prefix(materialized), version, false) else {
            return self
                .invalid_conversion(NativeScalarGetterKind::Boolean, nul_prefix(materialized));
        };
        let outcome = self.boolean_value(&number);
        convert(Some(NativeScalarCache::Number(number)), outcome)
    }

    fn fresh_boolean84(input: &[u8]) -> NativeScalarGetterConversion {
        let Some(value) = boolean84_word(input).or_else(|| {
            parse_number(input, TclVersion::V8_4, true)
                .map(|number| number_truth(&number))
                .or_else(|| float::parse_c_double(input).map(|number| number.value != 0.0))
        }) else {
            return Self::for_tcl_version(TclVersion::V8_4)
                .invalid_conversion(NativeScalarGetterKind::Boolean, input);
        };
        convert(
            Some(NativeScalarCache::WordBoolean(value)),
            Ok(NativeScalarGetterValue::Boolean(value)),
        )
    }

    fn wide_value(
        self,
        number: &Number,
    ) -> Result<NativeScalarGetterValue, NativeScalarGetterFailure> {
        match number {
            Number::Int(value) => Ok(NativeScalarGetterValue::Wide(*value)),
            Number::Big {
                negative,
                radix,
                digits,
            } if self
                .tcl_version()
                .is_some_and(|version| version < TclVersion::V9_0) =>
            {
                let magnitude = unsigned_magnitude(*radix, digits)
                    .ok_or(NativeScalarGetterFailure::IntegerOverflow)?;
                let value = magnitude.cast_signed();
                Ok(NativeScalarGetterValue::Wide(if *negative {
                    value.wrapping_neg()
                } else {
                    value
                }))
            }
            Number::Big { .. } => Err(NativeScalarGetterFailure::IntegerOverflow),
            _ => Err(NativeScalarGetterFailure::Invalid),
        }
    }

    fn double_value(
        self,
        number: &Number,
    ) -> Result<NativeScalarGetterValue, NativeScalarGetterFailure> {
        if (matches!(number, Number::Nan { .. })
            || matches!(number, Number::Double(value) if value.is_nan()))
            && self.engine != Engine::Tcl(TclVersion::V8_4)
            && !self.is_jim084()
        {
            return Err(NativeScalarGetterFailure::FloatingPointNaN);
        }
        Ok(NativeScalarGetterValue::Double(float::number_double(
            number,
        )))
    }

    fn boolean_value(
        self,
        number: &Number,
    ) -> Result<NativeScalarGetterValue, NativeScalarGetterFailure> {
        self.double_value(number)?;
        Ok(NativeScalarGetterValue::Boolean(number_truth(number)))
    }
}

fn convert(
    cache: Option<NativeScalarCache>,
    outcome: Result<NativeScalarGetterValue, NativeScalarGetterFailure>,
) -> NativeScalarGetterConversion {
    NativeScalarGetterConversion {
        cache,
        outcome,
        requires_string_materialization: false,
    }
}
fn invalid() -> NativeScalarGetterConversion {
    convert(None, Err(NativeScalarGetterFailure::Invalid))
}
fn cache_number(cache: &NativeScalarCache) -> Option<&Number> {
    if let NativeScalarCache::Number(number) = cache {
        Some(number)
    } else {
        None
    }
}
fn nul_prefix(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len())]
}
fn parse_number(input: &[u8], version: TclVersion, integer_only: bool) -> Option<Number> {
    let number = number::parse_whole_with(
        std::str::from_utf8(input).ok()?,
        ParseFlags {
            integer_only,
            ..ParseFlags::for_syntax(version.number_syntax())
        },
    )?;
    // Tcl's primitive integer parser rejects alphabetic floating values
    // before changing the original cache. The general numeral parser also
    // recognises NaN/Inf, so enforce this primitive-only requirement here.
    if integer_only && !matches!(number, Number::Int(_) | Number::Big { .. }) {
        return None;
    }
    Some(number)
}
fn unsigned_magnitude(radix: Radix, digits: &str) -> Option<u64> {
    digits.bytes().try_fold(0_u64, |value, byte| {
        value
            .checked_mul(radix as u64)?
            .checked_add(u64::from((byte as char).to_digit(radix as u32)?))
    })
}
fn number_truth(number: &Number) -> bool {
    match number {
        Number::Int(value) => *value != 0,
        Number::Big { .. } | Number::Nan { .. } => true,
        Number::Double(value) => *value != 0.0,
    }
}
fn boolean84_word(input: &[u8]) -> Option<bool> {
    if input.is_empty() {
        return None;
    }
    let mut lower = [0_u8; 10];
    for (index, byte) in input.iter().copied().take(9).enumerate() {
        if !byte.is_ascii() {
            return None;
        }
        lower[index] = byte.to_ascii_lowercase();
    }
    if lower[1] == 0 {
        match lower[0] {
            b'0' => return Some(false),
            b'1' => return Some(true),
            _ => {}
        }
    }
    for (word, value) in [
        (b"yes".as_slice(), true),
        (b"no", false),
        (b"true", true),
        (b"false", false),
        (b"on", true),
        (b"off", false),
    ] {
        if lower[0] == b'o' && input.len() < 2 {
            continue;
        }
        let matches = (0..input.len()).all(|index| {
            let left = lower.get(index).copied().unwrap_or(0);
            let right = word.get(index).copied().unwrap_or(0);
            left == right
        });
        // strncmp stops as soon as both sides contain their first NUL.
        let terminated_match = (0..input.len().min(lower.len()))
            .find(|&index| lower[index] == 0 && word.get(index).copied().unwrap_or(0) == 0)
            .is_some_and(|end| lower[..end] == word[..end.min(word.len())]);
        if matches || terminated_match {
            return Some(value);
        }
    }
    None
}

#[cfg(test)]
mod tests;
