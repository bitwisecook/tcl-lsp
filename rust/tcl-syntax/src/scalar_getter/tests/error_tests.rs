// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;

const STATE: [&str; 5] = [
    include_str!("../../../tests/data/native_scalar_getters/errors/error-state-8.4.20.txt"),
    include_str!("../../../tests/data/native_scalar_getters/errors/error-state-8.5.19.txt"),
    include_str!("../../../tests/data/native_scalar_getters/errors/error-state-8.6.18.txt"),
    include_str!("../../../tests/data/native_scalar_getters/errors/error-state-9.0.4.txt"),
    include_str!("../../../tests/data/native_scalar_getters/errors/error-state-9.1.0.txt"),
];
const OCTAL: [&str; 5] = [
    include_str!("../../../tests/data/native_scalar_getters/errors/octal-8.4.20.txt"),
    include_str!("../../../tests/data/native_scalar_getters/errors/octal-8.5.19.txt"),
    include_str!("../../../tests/data/native_scalar_getters/errors/octal-8.6.18.txt"),
    include_str!("../../../tests/data/native_scalar_getters/errors/octal-9.0.4.txt"),
    include_str!("../../../tests/data/native_scalar_getters/errors/octal-9.1.0.txt"),
];
const UNITS: [&str; 5] = [
    include_str!("../../../tests/data/native_scalar_getters/errors/units-8.4.20.txt"),
    include_str!("../../../tests/data/native_scalar_getters/errors/units-8.5.19.txt"),
    include_str!("../../../tests/data/native_scalar_getters/errors/units-8.6.18.txt"),
    include_str!("../../../tests/data/native_scalar_getters/errors/units-9.0.4.txt"),
    include_str!("../../../tests/data/native_scalar_getters/errors/units-9.1.0.txt"),
];

fn unhex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn kind(value: &str) -> NativeScalarGetterKind {
    match value {
        "0" => NativeScalarGetterKind::Wide,
        "1" => NativeScalarGetterKind::Double,
        "2" => NativeScalarGetterKind::Boolean,
        _ => panic!("unknown kind"),
    }
}
fn conversion(
    recipe: NativeScalarGetterProtocol,
    kind: NativeScalarGetterKind,
    storage: &str,
    bytes: &[u8],
) -> NativeScalarGetterConversion {
    let prior_kind = match storage {
        "2" => Some(NativeScalarGetterKind::Double),
        "3" => Some(NativeScalarGetterKind::Boolean),
        _ => None,
    };
    let prior = prior_kind
        .and_then(|kind| fixture_fresh_conversion(recipe, kind, bytes))
        .and_then(|conversion| conversion.cache().cloned());
    prior
        .as_ref()
        .map(|cache| recipe.cached_conversion(kind, cache, None))
        .transpose()
        .unwrap()
        .flatten()
        .or_else(|| fixture_fresh_conversion(recipe, kind, bytes))
        .unwrap()
}
fn verify_line(index: usize, line: &str, seeded: bool) {
    let recipe = protocol(index);
    let kind = kind(field(line, "getter"));
    let bytes = unhex(field(line, "input"));
    let conversion = conversion(recipe, kind, field(line, "storage"), &bytes);
    assert_eq!(
        conversion.outcome().is_ok(),
        field(line, "code") == "0",
        "engine {index}: {line}: {conversion:?}"
    );
    if let Err(failure) = conversion.outcome() {
        let error = recipe.failure_presentation(kind, failure, &bytes).unwrap();
        assert_eq!(
            error.message_bytes(),
            unhex(field(
                line,
                if index == 5 { "message" } else { "primitive" }
            )),
            "primitive engine {index}: {line}: {failure:?}"
        );
        assert_eq!(
            error.eval_message_bytes(),
            unhex(field(line, "message")),
            "propagated engine {index}: {line}"
        );
        // The probe duplicates only the primitive result. Its errorCode field
        // is sampled after the callback returns through actual Tcl_Eval.
        let update = match error.eval_error_code_update() {
            NativeScalarGetterErrorCode::Unchanged => error.error_code_update(),
            update => update,
        };
        let code = match update {
            NativeScalarGetterErrorCode::Unchanged => {
                if seeded {
                    b"PROBE BEFORE".as_slice()
                } else {
                    b"NONE".as_slice()
                }
            }
            NativeScalarGetterErrorCode::Set(bytes) => bytes.as_slice(),
        };
        assert_eq!(
            code,
            unhex(field(line, "errorCode")),
            "code engine {index}: {line}"
        );
    }
}

#[test]
fn primitive_failures_keep_cache_origin_and_existing_error_state() {
    // Native proof: naming.numeric.seeded-wide-frontier
    // docs/design/analysis/name-resolution-proofs/numeric-seeded-wide-frontier.md
    // Native proof: naming.numeric.seeded-double-frontier
    // docs/design/analysis/name-resolution-proofs/numeric-seeded-double-frontier.md
    // Native proof: naming.numeric.seeded-boolean-frontier
    // docs/design/analysis/name-resolution-proofs/numeric-seeded-boolean-frontier.md

    let mut count = 0;
    for (index, fixture) in STATE.iter().enumerate() {
        for line in fixture.lines() {
            verify_line(index, line, true);
            count += 1;
        }
    }
    for line in
        include_str!("../../../tests/data/native_scalar_getters/errors/jim-error-state.txt").lines()
    {
        verify_line(5, line, true);
        count += 1;
    }
    assert_eq!(count, 1113);
}

#[test]
fn invalid_octal_and_native_character_units_match_actual_releases() {
    // Native proof: naming.numeric.units-wide-frontier
    // docs/design/analysis/name-resolution-proofs/numeric-units-wide-frontier.md
    // Native proof: naming.numeric.units-double-frontier
    // docs/design/analysis/name-resolution-proofs/numeric-units-double-frontier.md
    // Native proof: naming.numeric.units-boolean-frontier
    // docs/design/analysis/name-resolution-proofs/numeric-units-boolean-frontier.md
    // Native proof: naming.numeric.octal-wide-frontier
    // docs/design/analysis/name-resolution-proofs/numeric-octal-wide-frontier.md
    // Native proof: naming.numeric.octal-double-frontier
    // docs/design/analysis/name-resolution-proofs/numeric-octal-double-frontier.md
    // Native proof: naming.numeric.octal-boolean-frontier
    // docs/design/analysis/name-resolution-proofs/numeric-octal-boolean-frontier.md

    let mut count = 0;
    for fixtures in [OCTAL, UNITS] {
        for (index, fixture) in fixtures.iter().enumerate() {
            for line in fixture.lines() {
                verify_line(index, line, false);
                count += 1;
            }
        }
    }
    assert_eq!(count, 390);
}

#[test]
fn unsupported_failure_stages_abstain_without_a_guest_diagnostic() {
    assert!(
        protocol(5)
            .failure_presentation(
                NativeScalarGetterKind::Double,
                NativeScalarGetterFailure::FloatingPointNaN,
                b"NaN"
            )
            .is_none()
    );
    assert!(
        protocol(0)
            .failure_presentation(
                NativeScalarGetterKind::Wide,
                NativeScalarGetterFailure::CachedNonInteger,
                b"1.5"
            )
            .is_none()
    );
    assert!(
        protocol(4)
            .failure_presentation(
                NativeScalarGetterKind::Double,
                NativeScalarGetterFailure::FloatingPointRange {
                    result_is_zero: false
                },
                b"1e400"
            )
            .is_none()
    );
}
