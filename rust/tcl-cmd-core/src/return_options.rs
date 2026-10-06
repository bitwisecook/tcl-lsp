//! Original-object completion-code conversion and return-option preparation.
//!
//! Adapters supply actual object getters and list storage. These helpers never
//! reconstruct a value from its spelling before reaching a native conversion.

use crate::{CmdError, CmdErrorCodeUpdate, CmdErrorDetails};

/// Selected native return command and completion-option protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnOptionsProtocol {
    /// Tcl 8.4 keyword-first sequential argv conversion.
    Tcl84,
    /// Tcl 8.5 dictionary merge and final control conversion.
    Tcl85,
    /// Tcl 8.6 and later, including checked error-stack options.
    Tcl86Plus,
    /// Current Jim sequential options and return-code cache.
    Jim084,
}

/// Cache installed by the native completion-code getter on its original object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionCodeCache {
    /// Tcl's exact completion keyword index; the original string stays resident.
    TclKeyword(i32),
    /// Jim's return-code object type, which has no string updater.
    Jim(i32),
}

/// Conversion purpose; an internal options dictionary is not user argv grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnOptionsPurpose {
    /// Exposed return command arguments.
    User,
    /// Bytecode or interpreter restoration of an options dictionary.
    InternalDictionary,
}

/// Actual object operations required by return and completion-code conversion.
pub trait ReturnOptionsOps {
    /// Original shared object reference retained by the concrete engine.
    type Value: Clone;
    /// Reach the selected native string door on the original object.
    fn bytes(&mut self, value: &Self::Value) -> Result<Vec<u8>, CmdError>;
    /// Reach original native list storage and retain its element objects.
    fn list(&mut self, value: &Self::Value) -> Result<Vec<Self::Value>, CmdError>;
    /// Create native string storage for a command that copies `CString` metadata.
    fn new_string(&mut self, bytes: &[u8]) -> Self::Value;
    /// Reach `GetInt` for C or `GetLong` for Jim with a NULL-interpreter probe.
    /// Guest conversion failure returns None; host refusal remains an error.
    fn integer_probe(&mut self, value: &Self::Value, wide: bool) -> Result<Option<i64>, CmdError>;
    /// Inspect only a cache compatible with the selected actual engine.
    fn completion_code_cache(&self, value: &Self::Value) -> Option<i32>;
    /// Install the authentic completion-code cache after successful conversion.
    fn adopt_completion_code_cache(
        &mut self,
        value: &Self::Value,
        cache: CompletionCodeCache,
    ) -> Result<(), CmdError>;
}

/// One option with its original key/value objects and exact comparison spelling.
#[derive(Debug, Clone)]
pub struct ReturnOptionPair<V> {
    /// Original key object.
    pub key: V,
    /// Original value object.
    pub value: V,
    /// Exact original key spelling; recognition remains protocol-selected.
    pub key_bytes: Vec<u8>,
}

impl<V> ReturnOptionPair<V> {
    /// Recognize this original key under the independently selected option grammar.
    #[must_use]
    pub fn name_in(&self, protocol: ReturnOptionsProtocol) -> &[u8] {
        protocol.name(&self.key_bytes)
    }
}

/// Checked even list, retaining the original objects rather than parsed strings.
#[derive(Debug, Clone)]
pub struct PreparedOptionPairs<V> {
    /// Original even key/value elements.
    pub pairs: Vec<ReturnOptionPair<V>>,
}

/// Prepared control and extra options. Result None means no result was supplied.
#[derive(Debug, Clone)]
pub struct PreparedReturn<V> {
    /// Requested native signed completion code.
    pub code: i32,
    /// Native return boundary count.
    pub level: i64,
    /// Original explicit result, if supplied.
    pub result: Option<V>,
    /// Original non-control option objects.
    pub options: Vec<ReturnOptionPair<V>>,
}

/// Prefix's upfront validation receipt, retaining the original options object.
#[derive(Debug, Clone)]
pub struct PreparedPrefixErrorOptions<V> {
    /// Original prefix options object.
    pub original: V,
    /// Whether native upfront cardinality permits suppressing a miss.
    pub suppresses_error: bool,
    /// C validates elements upfront; Jim deliberately validates byte length.
    pub c_pairs: Option<Vec<(V, V)>>,
}

impl ReturnOptionsProtocol {
    fn modern(self) -> bool {
        matches!(self, Self::Tcl85 | Self::Tcl86Plus)
    }
    fn name(self, bytes: &[u8]) -> &[u8] {
        if self.modern() {
            bytes
        } else {
            nul_prefix(bytes)
        }
    }
    pub(crate) fn error(self, message: Vec<u8>, code: &[u8]) -> CmdError {
        let error_code = match self {
            Self::Jim084 => CmdErrorCodeUpdate::Unchanged,
            Self::Tcl84 => CmdErrorCodeUpdate::Set(b"NONE".to_vec()),
            Self::Tcl85 | Self::Tcl86Plus => CmdErrorCodeUpdate::Set(code.to_vec()),
        };
        CmdError::from_byte_details(CmdErrorDetails {
            string_result: None,
            message,
            error_code,
            error_info: None,
            error_line: None,
            primitive_getter: None,
        })
    }
}

fn nul_prefix(bytes: &[u8]) -> &[u8] {
    &bytes[..bytes
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(bytes.len())]
}

fn quoted(prefix: &[u8], value: &[u8], suffix: &[u8]) -> Vec<u8> {
    let mut message = prefix.to_vec();
    message.extend_from_slice(nul_prefix(value));
    message.extend_from_slice(suffix);
    message
}

/// Convert an original completion-code object with the native cache schedule.
pub fn parse_completion_code<O: ReturnOptionsOps>(
    ops: &mut O,
    protocol: ReturnOptionsProtocol,
    value: &O::Value,
) -> Result<i32, CmdError> {
    if let Some(code) = ops.completion_code_cache(value) {
        return Ok(code);
    }
    // Tcl84 materializes argv before its keyword-first probe. Jim and modern
    // Tcl inspect a numeric representation before requesting any string.
    let spelling = if protocol == ReturnOptionsProtocol::Tcl84 {
        Some(ops.bytes(value)?)
    } else {
        None
    };
    if protocol != ReturnOptionsProtocol::Tcl84
        && let Some(number) = ops.integer_probe(value, protocol == ReturnOptionsProtocol::Jim084)?
    {
        let code = tcl_syntax::number::native_int32_low_bits(number);
        if protocol == ReturnOptionsProtocol::Jim084 {
            ops.adopt_completion_code_cache(value, CompletionCodeCache::Jim(code))?;
        }
        return Ok(code);
    }
    let bytes = match spelling {
        Some(bytes) => bytes,
        None => ops.bytes(value)?,
    };
    let names: &[&[u8]] = if protocol == ReturnOptionsProtocol::Jim084 {
        &[
            b"ok",
            b"error",
            b"return",
            b"break",
            b"continue",
            b"signal",
            b"exit",
            b"eval",
        ]
    } else {
        &[b"ok", b"error", b"return", b"break", b"continue"]
    };
    if let Some(index) = names.iter().position(|name| *name == nul_prefix(&bytes)) {
        let code = i32::try_from(index).expect("completion keyword index");
        match protocol {
            ReturnOptionsProtocol::Tcl84 => {}
            ReturnOptionsProtocol::Jim084 => {
                ops.adopt_completion_code_cache(value, CompletionCodeCache::Jim(code))?;
            }
            _ => ops.adopt_completion_code_cache(value, CompletionCodeCache::TclKeyword(code))?,
        }
        return Ok(code);
    }
    if protocol == ReturnOptionsProtocol::Tcl84
        && let Some(number) = ops.integer_probe(value, false)?
    {
        return Ok(tcl_syntax::number::native_int32_low_bits(number));
    }
    let message = if protocol == ReturnOptionsProtocol::Jim084 {
        quoted(b"expected return code but got \"", &bytes, b"\"")
    } else {
        quoted(
            b"bad completion code \"",
            &bytes,
            b"\": must be ok, error, return, break, continue, or an integer",
        )
    };
    Err(protocol.error(message, b"TCL RESULT ILLEGAL_CODE"))
}

/// Validate an options-list value using the selected original object-list door.
pub fn prepare_option_pairs<O: ReturnOptionsOps>(
    ops: &mut O,
    protocol: ReturnOptionsProtocol,
    original: &O::Value,
    purpose: ReturnOptionsPurpose,
) -> Result<PreparedOptionPairs<O::Value>, CmdError> {
    let items = match ops.list(original) {
        Ok(items) if items.len().is_multiple_of(2) => items,
        Err(error) if error.native_access_refusal().is_some() => return Err(error),
        _ => {
            let bytes = ops.bytes(original)?;
            let prefix: &[u8] = match purpose {
                ReturnOptionsPurpose::User => b"bad -options value: expected dictionary but got \"",
                ReturnOptionsPurpose::InternalDictionary => b"expected dict but got \"",
            };
            return Err(
                protocol.error(quoted(prefix, &bytes, b"\""), b"TCL RESULT ILLEGAL_OPTIONS")
            );
        }
    };
    let mut pairs = Vec::with_capacity(items.len() / 2);
    for pair in items.as_chunks::<2>().0 {
        pairs.push(ReturnOptionPair {
            key: pair[0].clone(),
            value: pair[1].clone(),
            key_bytes: ops.bytes(&pair[0])?,
        });
    }
    Ok(PreparedOptionPairs { pairs })
}

/// Prefix validates C's list cardinality or Jim's original byte cardinality.
pub fn prepare_prefix_error_options<O: ReturnOptionsOps>(
    ops: &mut O,
    protocol: ReturnOptionsProtocol,
    original: &O::Value,
) -> Result<PreparedPrefixErrorOptions<O::Value>, CmdError> {
    if protocol == ReturnOptionsProtocol::Jim084 {
        let bytes = ops.bytes(original)?;
        if !bytes.len().is_multiple_of(2) {
            return Err(protocol.error(
                b"error options must have an even number of elements".to_vec(),
                b"NONE",
            ));
        }
        return Ok(PreparedPrefixErrorOptions {
            original: original.clone(),
            suppresses_error: bytes.is_empty(),
            c_pairs: None,
        });
    }
    let items = ops.list(original)?;
    if !items.len().is_multiple_of(2) {
        return Err(protocol.error(
            b"error options must have an even number of elements".to_vec(),
            b"TCL VALUE DICTIONARY",
        ));
    }
    let pairs = items
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| (pair[0].clone(), pair[1].clone()))
        .collect::<Vec<_>>();
    Ok(PreparedPrefixErrorOptions {
        original: original.clone(),
        suppresses_error: pairs.is_empty(),
        c_pairs: Some(pairs),
    })
}

/// Parse user argv or an internal bytecode options application.
pub fn prepare_return<O: ReturnOptionsOps>(
    ops: &mut O,
    protocol: ReturnOptionsProtocol,
    args: &[O::Value],
    purpose: ReturnOptionsPurpose,
) -> Result<PreparedReturn<O::Value>, CmdError> {
    if purpose == ReturnOptionsPurpose::User && !protocol.modern() {
        return prepare_sequential_return(ops, protocol, args);
    }
    if purpose == ReturnOptionsPurpose::InternalDictionary
        && args.len() >= 2
        && ops.bytes(&args[0])? == b"-options"
    {
        // Bytecode carries one outer dictionary, whose ingestion error is
        // distinct from a nested user -options option.
        let _ = prepare_option_pairs(
            ops,
            protocol,
            &args[1],
            ReturnOptionsPurpose::InternalDictionary,
        )?;
    }
    let pair_len = args.len() & !1;
    let result = args.get(pair_len).cloned();
    let mut pairs = Vec::with_capacity(pair_len / 2);
    for pair in args[..pair_len].as_chunks::<2>().0 {
        let key_bytes = ops.bytes(&pair[0])?;
        if protocol == ReturnOptionsProtocol::Tcl84 {
            let _ = ops.bytes(&pair[1])?;
        }
        pairs.push(ReturnOptionPair {
            key: pair[0].clone(),
            value: pair[1].clone(),
            key_bytes,
        });
    }
    prepare_return_pairs(
        ops,
        protocol,
        PreparedOptionPairs { pairs },
        result,
        purpose,
    )
}

fn prepare_sequential_return<O: ReturnOptionsOps>(
    ops: &mut O,
    protocol: ReturnOptionsProtocol,
    args: &[O::Value],
) -> Result<PreparedReturn<O::Value>, CmdError> {
    let pair_len = args.len() & !1;
    let mut prepared = PreparedReturn {
        code: 0,
        level: 1,
        result: args.get(pair_len).cloned(),
        options: Vec::new(),
    };
    for pair in args[..pair_len].as_chunks::<2>().0 {
        let key_bytes = ops.bytes(&pair[0])?;
        if protocol == ReturnOptionsProtocol::Tcl84 {
            let _ = ops.bytes(&pair[1])?;
        }
        let pair = ReturnOptionPair {
            key: pair[0].clone(),
            value: pair[1].clone(),
            key_bytes,
        };
        apply_sequential_pair(ops, protocol, pair, &mut prepared)?;
    }
    Ok(prepared)
}

fn apply_sequential_pair<O: ReturnOptionsOps>(
    ops: &mut O,
    protocol: ReturnOptionsProtocol,
    pair: ReturnOptionPair<O::Value>,
    prepared: &mut PreparedReturn<O::Value>,
) -> Result<(), CmdError> {
    let mut pair = pair;
    let key = protocol.name(&pair.key_bytes);
    match key {
        b"-code" => {
            prepared.code = parse_completion_code(ops, protocol, &pair.value)?;
            return Ok(());
        }
        b"-level" if protocol == ReturnOptionsProtocol::Jim084 => {
            prepared.level = parse_level(ops, protocol, &pair.value)?;
            return Ok(());
        }
        b"-errorcode" | b"-errorinfo" => {}
        _ => {
            let message = if protocol == ReturnOptionsProtocol::Jim084 {
                b"wrong # args: should be \"return ?-code code? ?-errorinfo stacktrace? ?-level level? ?result?\"".to_vec()
            } else {
                quoted(
                    b"bad option \"",
                    &pair.key_bytes,
                    b"\": must be -code, -errorcode, or -errorinfo",
                )
            };
            return Err(protocol.error(message, b"TCL WRONGARGS"));
        }
    }
    if protocol == ReturnOptionsProtocol::Tcl84 {
        let bytes = ops.bytes(&pair.value)?;
        pair.value = ops.new_string(nul_prefix(&bytes));
    }
    if let Some(slot) = prepared
        .options
        .iter_mut()
        .find(|slot| protocol.name(&slot.key_bytes) == key)
    {
        slot.value = pair.value;
    } else {
        prepared.options.push(pair);
    }
    Ok(())
}

/// Apply already checked pairs, preserving the caller's original option objects.
pub fn prepare_return_pairs<O: ReturnOptionsOps>(
    ops: &mut O,
    protocol: ReturnOptionsProtocol,
    prepared: PreparedOptionPairs<O::Value>,
    result: Option<O::Value>,
    purpose: ReturnOptionsPurpose,
) -> Result<PreparedReturn<O::Value>, CmdError> {
    if !protocol.modern() && purpose == ReturnOptionsPurpose::User {
        let mut result = PreparedReturn {
            code: 0,
            level: 1,
            result,
            options: Vec::new(),
        };
        for pair in prepared.pairs {
            apply_sequential_pair(ops, protocol, pair, &mut result)?;
        }
        return Ok(result);
    }
    let merge = protocol.modern() || purpose == ReturnOptionsPurpose::InternalDictionary;
    let mut options: Vec<ReturnOptionPair<O::Value>> = Vec::new();
    let mut code = 0;
    let mut level = 1;
    let mut work: Vec<_> = prepared.pairs.into_iter().rev().collect();
    while let Some(pair) = work.pop() {
        let key = protocol.name(&pair.key_bytes);
        if merge && key == b"-options" {
            let nested =
                prepare_option_pairs(ops, protocol, &pair.value, ReturnOptionsPurpose::User)?;
            work.extend(nested.pairs.into_iter().rev());
            continue;
        }
        if let Some(slot) = options
            .iter_mut()
            .find(|slot| protocol.name(&slot.key_bytes) == key)
        {
            slot.value = pair.value;
        } else {
            options.push(pair);
        }
    }
    if merge {
        if let Some(index) = options
            .iter()
            .position(|pair| protocol.name(&pair.key_bytes) == b"-code")
        {
            code = parse_completion_code(ops, protocol, &options.remove(index).value)?;
        }
        if let Some(index) = options
            .iter()
            .position(|pair| protocol.name(&pair.key_bytes) == b"-level")
        {
            level = parse_level(ops, protocol, &options.remove(index).value)?;
        }
        if protocol.modern()
            && let Some(pair) = options.iter().find(|pair| pair.key_bytes == b"-errorcode")
        {
            validate_list(ops, protocol, &pair.value, false)?;
        }
        if protocol == ReturnOptionsProtocol::Tcl86Plus
            && let Some(pair) = options.iter().find(|pair| pair.key_bytes == b"-errorstack")
        {
            validate_list(ops, protocol, &pair.value, true)?;
        }
        if protocol.modern() && code == 2 {
            code = 0;
            level = i64::from(tcl_syntax::number::native_int32_low_bits(level).wrapping_add(1));
        }
    }
    Ok(PreparedReturn {
        code,
        level,
        result,
        options,
    })
}

pub(crate) fn parse_level<O: ReturnOptionsOps>(
    ops: &mut O,
    protocol: ReturnOptionsProtocol,
    value: &O::Value,
) -> Result<i64, CmdError> {
    if let Some(number) = ops.integer_probe(value, protocol == ReturnOptionsProtocol::Jim084)?
        && number >= 0
    {
        return Ok(number);
    }
    let bytes = ops.bytes(value)?;
    let message = if protocol == ReturnOptionsProtocol::Jim084 {
        quoted(b"bad level \"", &bytes, b"\"")
    } else {
        quoted(
            b"bad -level value: expected non-negative integer but got \"",
            &bytes,
            b"\"",
        )
    };
    Err(protocol.error(message, b"TCL RESULT ILLEGAL_LEVEL"))
}

pub(crate) fn validate_list<O: ReturnOptionsOps>(
    ops: &mut O,
    protocol: ReturnOptionsProtocol,
    value: &O::Value,
    stack: bool,
) -> Result<(), CmdError> {
    match ops.list(value) {
        Ok(items) if !stack || items.len().is_multiple_of(2) => Ok(()),
        Err(error) if error.native_access_refusal().is_some() => Err(error),
        result => {
            let bytes = ops.bytes(value)?;
            let (prefix, code): (&[u8], &[u8]) = match (stack, result.is_ok()) {
                (false, _) => (
                    b"bad -errorcode value: expected a list but got \"",
                    b"TCL RESULT ILLEGAL_ERRORCODE",
                ),
                (true, false) => (
                    b"bad -errorstack value: expected a list but got \"",
                    b"TCL RESULT NONLIST_ERRORSTACK",
                ),
                (true, true) => (
                    b"forbidden odd-sized list for -errorstack: \"",
                    b"TCL RESULT ODDSIZEDLIST_ERRORSTACK",
                ),
            };
            Err(protocol.error(quoted(prefix, &bytes, b"\""), code))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};

    #[derive(Clone, Debug)]
    struct Object(Rc<ObjectData>);
    #[derive(Debug)]
    struct ObjectData {
        bytes: Option<Vec<u8>>,
        integer: Option<i64>,
        items: Option<Vec<Object>>,
        reads: Cell<usize>,
        probes: Cell<usize>,
        code: Cell<Option<CompletionCodeCache>>,
    }
    impl Object {
        fn text(bytes: &[u8]) -> Self {
            Self::make(Some(bytes.to_vec()), None, None)
        }
        fn integer(value: i64) -> Self {
            Self::make(None, Some(value), None)
        }
        fn list(items: Vec<Self>) -> Self {
            Self::make(None, None, Some(items))
        }
        fn make(bytes: Option<Vec<u8>>, integer: Option<i64>, items: Option<Vec<Self>>) -> Self {
            Self(Rc::new(ObjectData {
                bytes,
                integer,
                items,
                reads: Cell::new(0),
                probes: Cell::new(0),
                code: Cell::new(None),
            }))
        }
    }
    struct Ops;
    impl ReturnOptionsOps for Ops {
        type Value = Object;
        fn bytes(&mut self, value: &Object) -> Result<Vec<u8>, CmdError> {
            value.0.reads.set(value.0.reads.get() + 1);
            value.0.bytes.clone().ok_or_else(|| {
                CmdError::from(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "test object has no updater",
                    ),
                )
            })
        }
        fn list(&mut self, value: &Object) -> Result<Vec<Object>, CmdError> {
            value
                .0
                .items
                .clone()
                .ok_or_else(|| CmdError::new("unmatched open brace in list"))
        }
        fn new_string(&mut self, bytes: &[u8]) -> Object {
            Object::text(bytes)
        }
        fn integer_probe(&mut self, value: &Object, _: bool) -> Result<Option<i64>, CmdError> {
            value.0.probes.set(value.0.probes.get() + 1);
            Ok(value.0.integer)
        }
        fn completion_code_cache(&self, value: &Object) -> Option<i32> {
            value.0.code.get().map(|cache| match cache {
                CompletionCodeCache::TclKeyword(code) | CompletionCodeCache::Jim(code) => code,
            })
        }
        fn adopt_completion_code_cache(
            &mut self,
            value: &Object,
            cache: CompletionCodeCache,
        ) -> Result<(), CmdError> {
            value.0.code.set(Some(cache));
            Ok(())
        }
    }

    #[test]
    fn modern_merge_converts_only_last_control_and_keeps_original_custom_key() {
        let bad = Object::text(b"bad");
        let code = Object::text(b"ok");
        let key = Object::text(b"-custom\xff\0tail");
        let value = Object::text(b"VALUE");
        let nested = Object::list(vec![
            Object::text(b"-code"),
            bad.clone(),
            key.clone(),
            value.clone(),
        ]);
        let prepared = prepare_return(
            &mut Ops,
            ReturnOptionsProtocol::Tcl86Plus,
            &[
                Object::text(b"-options"),
                nested,
                Object::text(b"-code"),
                code.clone(),
            ],
            ReturnOptionsPurpose::User,
        )
        .unwrap();
        assert_eq!(prepared.code, 0);
        assert_eq!(bad.0.probes.get(), 0);
        assert_eq!(code.0.code.get(), Some(CompletionCodeCache::TclKeyword(0)));
        assert_eq!(prepared.options[0].key_bytes, b"-custom\xff\0tail");
        assert!(Rc::ptr_eq(&prepared.options[0].key.0, &key.0));
        assert!(Rc::ptr_eq(&prepared.options[0].value.0, &value.0));
    }

    #[test]
    fn sequential_failure_does_not_prepare_later_pair() {
        for protocol in [ReturnOptionsProtocol::Tcl84, ReturnOptionsProtocol::Jim084] {
            let later = Object::text(b"-code");
            let error = prepare_return(
                &mut Ops,
                protocol,
                &[
                    Object::text(b"-code"),
                    Object::text(b"bad"),
                    later.clone(),
                    Object::text(b"ok"),
                ],
                ReturnOptionsPurpose::User,
            );
            assert!(error.is_err());
            assert_eq!(later.0.reads.get(), 0);
        }
    }

    #[test]
    fn jim_numeric_completion_code_has_no_string_preparation() {
        let value = Object::integer(1);
        assert_eq!(
            parse_completion_code(&mut Ops, ReturnOptionsProtocol::Jim084, &value).unwrap(),
            1
        );
        assert_eq!(value.0.reads.get(), 0);
        assert_eq!(value.0.code.get(), Some(CompletionCodeCache::Jim(1)));
        assert_eq!(
            parse_completion_code(&mut Ops, ReturnOptionsProtocol::Jim084, &value).unwrap(),
            1
        );
        assert_eq!(value.0.probes.get(), 1);
    }

    #[test]
    fn prefix_validation_preserves_native_cardinality_and_does_not_read_c_keys() {
        let key = Object::integer(9);
        let c = Object::list(vec![key.clone(), Object::text(b"VALUE")]);
        let receipt =
            prepare_prefix_error_options(&mut Ops, ReturnOptionsProtocol::Tcl86Plus, &c).unwrap();
        assert!(!receipt.suppresses_error);
        assert_eq!(key.0.reads.get(), 0);
        assert!(
            prepare_prefix_error_options(
                &mut Ops,
                ReturnOptionsProtocol::Jim084,
                &Object::text(b"-x")
            )
            .is_ok()
        );
        assert!(
            prepare_prefix_error_options(
                &mut Ops,
                ReturnOptionsProtocol::Jim084,
                &Object::text(b"-x 00")
            )
            .is_err()
        );
    }

    #[test]
    fn modern_return_code_adds_a_boundary_but_jim_keeps_requested_code() {
        for (protocol, code, level) in [
            (ReturnOptionsProtocol::Tcl86Plus, 0, 1),
            (ReturnOptionsProtocol::Jim084, 2, 0),
        ] {
            let prepared = prepare_return(
                &mut Ops,
                protocol,
                &[
                    Object::text(b"-code"),
                    Object::text(b"return"),
                    Object::text(b"-level"),
                    Object::integer(0),
                ],
                ReturnOptionsPurpose::User,
            )
            .unwrap();
            assert_eq!((prepared.code, prepared.level), (code, level));
        }
    }
}
