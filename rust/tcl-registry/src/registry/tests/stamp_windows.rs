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

//! Release-windowed stamps: the hook, semantic operation or native lowering selected at one dialect point.

use super::*;

// Versioned codegen-axis stamps: selected at the point a call is resolved
// at, and declined — never guessed — where that point does not settle them.

const HOOK_FROM_9: &[crate::stamp_window::StampWindow<crate::hooks::CodegenHookId>] =
    &[crate::stamp_window::StampWindow {
        lifecycle: crate::lifecycle::Lifecycle::introduced_in("9.0"),
        value: crate::hooks::CodegenHookId::Lassign,
    }];

const INLINE_BEFORE_9: &[crate::stamp_window::StampWindow<crate::hooks::InlineCodegenHookId>] =
    &[crate::stamp_window::StampWindow {
        lifecycle: crate::lifecycle::Lifecycle::UNSPECIFIED.retired_from("9.0"),
        value: crate::hooks::InlineCodegenHookId::Expr,
    }];

pub(super) fn points() -> [(&'static str, Option<SurfaceQuery<'static>>); 5] {
    [
        ("8.6", Some(SurfaceQuery::core(Family::Tcl, "8.6"))),
        ("9.0", Some(SurfaceQuery::core(Family::Tcl, "9.0"))),
        ("9.1", Some(SurfaceQuery::core(Family::Tcl, "9.1"))),
        (
            "the whole ladder",
            Some(SurfaceQuery::any_release(Family::Tcl)),
        ),
        ("no point", None),
    ]
}

#[test]
fn a_stamp_window_is_selected_at_the_primary_release_and_declines_where_it_is_not_settled() {
    use crate::hooks::{CodegenHookId, InlineCodegenHookId};

    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "stamp::probe",
        codegen_hook_windows: HOOK_FROM_9,
        inline_codegen_hook_windows: INLINE_BEFORE_9,
        ..CommandSpec::DEFAULT
    });
    for (point, query, codegen, inline) in [
        ("8.6", points()[0].1, None, Some(InlineCodegenHookId::Expr)),
        ("9.0", points()[1].1, Some(CodegenHookId::Lassign), None),
        ("9.1", points()[2].1, Some(CodegenHookId::Lassign), None),
        ("the whole ladder", points()[3].1, None, None),
        ("no point", points()[4].1, None, None),
    ] {
        let call = registry
            .resolve_call("stamp::probe", &["$l", "a"], query)
            .expect("the probe resolves");
        assert_eq!(call.codegen_hook, codegen, "codegen hook at {point}");
        assert_eq!(call.inline_codegen_hook, inline, "inline hook at {point}");
    }
}

#[test]
fn the_plain_stamp_stands_where_no_window_covers_and_a_window_wins_where_one_does() {
    use crate::hooks::CodegenHookId;

    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "stamp::beside",
        codegen_hook: Some(CodegenHookId::Llength),
        codegen_hook_windows: HOOK_FROM_9,
        ..CommandSpec::DEFAULT
    });
    for (point, query, codegen) in [
        ("8.6", points()[0].1, Some(CodegenHookId::Llength)),
        ("9.0", points()[1].1, Some(CodegenHookId::Lassign)),
        ("the whole ladder", points()[3].1, None),
    ] {
        let call = registry
            .resolve_call("stamp::beside", &["$l"], query)
            .expect("the probe resolves");
        assert_eq!(call.codegen_hook, codegen, "{point}");
    }
}

/// A subcommand whose windows state nothing at the release is the command's
/// own call; one whose windows the point does not settle is dispatched
/// plain, and the command's hook is not the answer in its place.
#[test]
fn a_subcommand_inherits_where_its_windows_are_silent_and_does_not_where_they_decline() {
    use crate::hooks::CodegenHookId;

    let sub = SubCommand {
        name: "get",
        codegen_hook_windows: HOOK_FROM_9,
        ..SubCommand::DEFAULT
    };
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "stamp::ensemble",
        codegen_hook: Some(CodegenHookId::Dict),
        subcommands: Box::leak(Box::new([sub])),
        ..CommandSpec::DEFAULT
    });
    for (point, query, codegen) in [
        ("8.6", points()[0].1, Some(CodegenHookId::Dict)),
        ("9.0", points()[1].1, Some(CodegenHookId::Lassign)),
        ("the whole ladder", points()[3].1, None),
        ("no point", points()[4].1, None),
    ] {
        let call = registry
            .resolve_call("stamp::ensemble", &["get", "$d", "k"], query)
            .expect("the ensemble resolves");
        assert_eq!(call.codegen_hook, codegen, "{point}");
        // The command's own call is not the subcommand's to decline.
        let own = registry
            .resolve_call("stamp::ensemble", &["$d"], query)
            .expect("the command resolves");
        assert_eq!(own.codegen_hook, Some(CodegenHookId::Dict), "{point}");
    }
}

#[test]
fn a_semantic_operation_window_decides_the_operation_a_resolved_invocation_has() {
    use crate::intrinsic::IntrinsicId;
    use crate::semantic_operation::SemanticOperationId;
    use crate::stamp_window::StampWindow;

    const LENGTH_FROM_9: &[StampWindow<SemanticOperationId>] = &[StampWindow {
        lifecycle: crate::lifecycle::Lifecycle::introduced_in("9.0"),
        value: SemanticOperationId::Intrinsic(IntrinsicId::StringLength),
    }];
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "stamp::length",
        semantic_operation_windows: LENGTH_FROM_9,
        ..CommandSpec::DEFAULT
    });
    for (point, query, operation) in [
        ("8.6", points()[0].1, SemanticOperationId::Invoke),
        (
            "9.0",
            points()[1].1,
            SemanticOperationId::Intrinsic(IntrinsicId::StringLength),
        ),
        (
            "the whole ladder",
            points()[3].1,
            SemanticOperationId::Invoke,
        ),
        ("no point", points()[4].1, SemanticOperationId::Invoke),
    ] {
        let invocation = registry
            .resolve_invocation("stamp::length", &["abc"], query)
            .expect("the probe resolves");
        assert_eq!(invocation.semantics.operation, operation, "{point}");
    }
}

/// A subcommand whose operation windows state nothing at the release is the
/// command's own operation; one whose windows the point does not settle is
/// a plain invoke, and the command's operation is not the answer instead.
#[test]
fn a_subcommand_operation_inherits_where_silent_and_is_plain_where_it_declines() {
    use crate::intrinsic::IntrinsicId;
    use crate::semantic_operation::SemanticOperationId;
    use crate::stamp_window::StampWindow;

    const LENGTH_FROM_9: &[StampWindow<SemanticOperationId>] = &[StampWindow {
        lifecycle: crate::lifecycle::Lifecycle::introduced_in("9.0"),
        value: SemanticOperationId::Intrinsic(IntrinsicId::StringLength),
    }];
    let concat = SemanticOperationId::Intrinsic(IntrinsicId::Concat);
    let size = SubCommand {
        name: "size",
        semantic_operation_windows: LENGTH_FROM_9,
        ..SubCommand::DEFAULT
    };
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "stamp::inherits",
        semantic_operation: Some(concat),
        subcommands: Box::leak(Box::new([size])),
        ..CommandSpec::DEFAULT
    });
    for (point, query, operation) in [
        ("8.6", points()[0].1, concat),
        (
            "9.0",
            points()[1].1,
            SemanticOperationId::Intrinsic(IntrinsicId::StringLength),
        ),
        (
            "the whole ladder",
            points()[3].1,
            SemanticOperationId::Invoke,
        ),
        ("no point", points()[4].1, SemanticOperationId::Invoke),
    ] {
        let invocation = registry
            .resolve_invocation("stamp::inherits", &["size", "abc"], query)
            .expect("the ensemble resolves");
        assert_eq!(invocation.semantics.operation, operation, "{point}");
    }
}

/// The operation a codegen or inline hook names is read through the same
/// selection: a window gives it where it covers the release, and a level
/// that declines gives plain dispatch, not its command's operation.
#[test]
fn the_operation_a_windowed_hook_names_is_selected_and_declined_like_the_hook() {
    use crate::hooks::{CodegenHookId, InlineCodegenHookId};
    use crate::intrinsic::IntrinsicId;
    use crate::lifecycle::Lifecycle;
    use crate::semantic_operation::SemanticOperationId;
    use crate::stamp_window::StampWindow;

    const LLENGTH_FROM_9: &[StampWindow<CodegenHookId>] = &[StampWindow {
        lifecycle: Lifecycle::introduced_in("9.0"),
        value: CodegenHookId::Llength,
    }];
    const LINDEX_FROM_9: &[StampWindow<InlineCodegenHookId>] = &[StampWindow {
        lifecycle: Lifecycle::introduced_in("9.0"),
        value: InlineCodegenHookId::Lindex,
    }];
    let concat = SemanticOperationId::Intrinsic(IntrinsicId::Concat);
    let by_codegen = SubCommand {
        name: "count",
        codegen_hook_windows: LLENGTH_FROM_9,
        ..SubCommand::DEFAULT
    };
    let by_inline = SubCommand {
        name: "at",
        inline_codegen_hook_windows: LINDEX_FROM_9,
        ..SubCommand::DEFAULT
    };
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "stamp::hooked",
        semantic_operation: Some(concat),
        subcommands: Box::leak(Box::new([by_codegen, by_inline])),
        ..CommandSpec::DEFAULT
    });
    let operation = |sub: &str, query| {
        registry
            .resolve_invocation("stamp::hooked", &[sub, "abc"], query)
            .expect("the ensemble resolves")
            .semantics
            .operation
    };
    for (sub, at_9) in [
        (
            "count",
            SemanticOperationId::Intrinsic(IntrinsicId::ListLength),
        ),
        ("at", SemanticOperationId::Intrinsic(IntrinsicId::ListIndex)),
    ] {
        assert_eq!(
            operation(sub, points()[0].1),
            concat,
            "{sub} at 8.6 inherits"
        );
        assert_eq!(operation(sub, points()[1].1), at_9, "{sub} at 9.0");
        for (point, query) in [
            ("the whole ladder", points()[3].1),
            ("no point", points()[4].1),
        ] {
            assert_eq!(
                operation(sub, query),
                SemanticOperationId::Invoke,
                "{sub} at {point} declines"
            );
        }
    }
}

#[test]
fn a_native_lowering_window_is_read_at_the_point_and_declines_across_its_edge() {
    use crate::completion::CompletionCode;
    use crate::lifecycle::Lifecycle;
    use crate::native_lowering::NativeLowering;
    use crate::stamp_window::StampWindow;

    const BREAK_FROM_9: &[StampWindow<NativeLowering>] = &[StampWindow {
        lifecycle: Lifecycle::introduced_in("9.0"),
        value: NativeLowering::Completion(CompletionCode::Break),
    }];
    let spec = CommandSpec {
        name: "stamp::native",
        native_lowering_windows: BREAK_FROM_9,
        ..CommandSpec::DEFAULT
    };
    let shapes = points().map(|(_, query)| spec.native_lowering_at(query.as_ref()));
    assert_eq!(
        shapes,
        [
            NativeLowering::Generic,
            NativeLowering::Completion(CompletionCode::Break),
            NativeLowering::Completion(CompletionCode::Break),
            NativeLowering::Generic,
            NativeLowering::Generic,
        ]
    );
    assert_eq!(
        spec.native_lowering(),
        NativeLowering::Generic,
        "the plain accessor reads the plain field alone"
    );
}

#[test]
fn native_lowering_descriptor_metadata_keeps_exact_keys_and_supplied_stamp_windows() {
    use crate::completion::CompletionCode;
    use crate::lifecycle::Lifecycle;
    use crate::native_lowering::NativeLowering;
    use crate::stamp_window::StampWindow;

    const BREAK_FROM_9: &[StampWindow<NativeLowering>] = &[StampWindow {
        lifecycle: Lifecycle::introduced_in("9.0"),
        value: NativeLowering::Completion(CompletionCode::Break),
    }];
    let selected = CommandSpec {
        name: "stamp::native",
        native_lowering_windows: BREAK_FROM_9,
        ..CommandSpec::DEFAULT
    };
    let rooted = CommandSpec {
        name: "::stamp::native",
        native_lowering: Some(NativeLowering::Completion(CompletionCode::Continue)),
        ..CommandSpec::DEFAULT
    };
    let mut registry = CommandRegistry::build_default();
    registry.insert(selected.clone());
    assert_eq!(
        registry.native_lowering_for_descriptor(rooted.descriptor_key(), None),
        None,
        "an exact descriptor cannot borrow the unrooted row",
    );
    registry.insert(rooted.clone());
    for (_, query) in points() {
        assert_eq!(
            registry.native_lowering_for_descriptor(selected.descriptor_key(), query.as_ref()),
            Some(selected.native_lowering_at(query.as_ref())),
        );
        assert_eq!(
            registry.native_lowering_for_descriptor(rooted.descriptor_key(), query.as_ref()),
            Some(rooted.native_lowering_at(query.as_ref())),
        );
    }
}

/// Every operation a windowed stamp could give a command counts when the
/// question is surface-blind: which names could be this operation.
#[test]
fn a_windowed_operation_is_counted_by_the_questions_that_ask_no_release() {
    use crate::intrinsic::IntrinsicId;
    use crate::semantic_operation::SemanticOperationId;
    use crate::stamp_window::StampWindow;

    const LENGTH_FROM_9: &[StampWindow<SemanticOperationId>] = &[StampWindow {
        lifecycle: crate::lifecycle::Lifecycle::introduced_in("9.0"),
        value: SemanticOperationId::Intrinsic(IntrinsicId::StringLength),
    }];
    let spec = CommandSpec {
        name: "stamp::length",
        semantic_operation_windows: LENGTH_FROM_9,
        ..CommandSpec::DEFAULT
    };
    assert!(spec.intrinsic_ids().contains(&IntrinsicId::StringLength));
    let mut registry = CommandRegistry::build_default();
    registry.insert(spec);
    assert!(
        registry
            .command_names_for_semantic_operation(SemanticOperationId::Intrinsic(
                IntrinsicId::StringLength
            ))
            .any(|name| name == "stamp::length")
    );
    assert!(
        !CommandSpec {
            name: "stamp::plain",
            ..CommandSpec::DEFAULT
        }
        .intrinsic_ids()
        .contains(&IntrinsicId::StringLength)
    );
}
