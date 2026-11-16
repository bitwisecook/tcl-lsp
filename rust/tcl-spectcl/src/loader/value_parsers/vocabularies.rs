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

//! The closed vocabularies a pack names by variant: completion codes, types, side-effect targets, frame layouts, hooks and intrinsics.

use super::super::{Log, leak_slice};
use super::{enum_by_name, list_words};
use tcl_registry::body_kind::BodyKind;
use tcl_registry::byte_array_effect::ByteArrayEffect;
use tcl_registry::completion::{CompletionCode, CompletionDescriptor};
use tcl_registry::frame_effect::{FrameArgLayout, FrameLevelWord};
use tcl_registry::hooks::{
    AnalyserHookId, CodegenHookId, InlineCodegenHookId, LoweringHookId, ReturnTypeHookId,
};
use tcl_registry::hover::{FormKind, ScriptTiming, VariableScope};
use tcl_registry::intrinsic::IntrinsicId;
use tcl_registry::patterns::FormatType;
use tcl_registry::presentation::ArgPresentation;
use tcl_registry::side_effects::{ConnectionSide, SideEffectTarget, StorageType};
use tcl_registry::spec::DefaultFormFirstWord;
use tcl_registry::types::TclType;

const COMPLETION_CODES: &[CompletionCode] = &[
    CompletionCode::Ok,
    CompletionCode::Error,
    CompletionCode::Return,
    CompletionCode::Break,
    CompletionCode::Continue,
];

/// `completion {CODE …}`: the exact completion codes the command can finish
/// with, each producing its result and options. An unreadable row is dropped,
/// leaving the conservative descriptor.
pub(in super::super) fn completion_row(
    text: &str,
    line: u32,
    log: &mut Log,
) -> Option<CompletionDescriptor> {
    let mut codes = Vec::new();
    for word in list_words(text) {
        let code = enum_by_name(COMPLETION_CODES, &word, "completion code", line, log)?;
        if !codes.contains(&code) {
            codes.push(code);
        }
    }
    if codes.is_empty() {
        log.say(line, "an empty `completion` row is dropped");
        return None;
    }
    Some(CompletionDescriptor::exact(leak_slice(codes)))
}

pub(in super::super) const TCL_TYPES: &[TclType] = &[
    TclType::String,
    TclType::Int,
    TclType::Double,
    TclType::Boolean,
    TclType::List,
    TclType::Dict,
    TclType::ByteArray,
    TclType::Numeric,
    TclType::Object,
    TclType::Channel,
];

pub(in super::super) const SIDE_EFFECT_TARGETS: &[SideEffectTarget] = &[
    SideEffectTarget::Variable,
    SideEffectTarget::SessionTable,
    SideEffectTarget::PersistenceTable,
    SideEffectTarget::DataGroup,
    SideEffectTarget::HttpHeader,
    SideEffectTarget::HttpBody,
    SideEffectTarget::HttpStatus,
    SideEffectTarget::HttpUri,
    SideEffectTarget::HttpCookie,
    SideEffectTarget::HttpMethod,
    SideEffectTarget::Http2State,
    SideEffectTarget::ResponseCommit,
    SideEffectTarget::ConnectionControl,
    SideEffectTarget::TcpState,
    SideEffectTarget::SslState,
    SideEffectTarget::UdpState,
    SideEffectTarget::PoolSelection,
    SideEffectTarget::NodeSelection,
    SideEffectTarget::SnatSelection,
    SideEffectTarget::FileIo,
    SideEffectTarget::NetworkIo,
    SideEffectTarget::LogIo,
    SideEffectTarget::StreamProfile,
    SideEffectTarget::DnsState,
    SideEffectTarget::ClassificationState,
    SideEffectTarget::Dosl7State,
    SideEffectTarget::FlowState,
    SideEffectTarget::LsnState,
    SideEffectTarget::FtpState,
    SideEffectTarget::IcapState,
    SideEffectTarget::MessageState,
    SideEffectTarget::IStats,
    SideEffectTarget::ApmState,
    SideEffectTarget::AsmState,
    SideEffectTarget::BigipConfig,
    SideEffectTarget::ProcDefinition,
    SideEffectTarget::NamespaceState,
    SideEffectTarget::InterpState,
    SideEffectTarget::Process,
    SideEffectTarget::ChannelIo,
    SideEffectTarget::EventControl,
    SideEffectTarget::Unknown,
];

pub(in super::super) const CONNECTION_SIDES: &[ConnectionSide] = &[
    ConnectionSide::None,
    ConnectionSide::Client,
    ConnectionSide::Server,
    ConnectionSide::Both,
    ConnectionSide::Global,
];

pub(in super::super) const FORM_KINDS: &[FormKind] =
    &[FormKind::Default, FormKind::Getter, FormKind::Setter];

pub(in super::super) const BODY_KINDS: &[BodyKind] = &[BodyKind::Plain, BodyKind::Structural];

pub(in super::super) const SCRIPT_TIMINGS: &[ScriptTiming] = &[
    ScriptTiming::SameInvocation,
    ScriptTiming::Deferred,
    ScriptTiming::ReferenceOnly,
];

pub(in super::super) const VARIABLE_SCOPES: &[VariableScope] =
    &[VariableScope::CurrentFrame, VariableScope::Global];

pub(super) const BYTE_ARRAY_EFFECTS: &[ByteArrayEffect] = &[
    ByteArrayEffect::None,
    ByteArrayEffect::Transparent,
    ByteArrayEffect::Coerces,
    ByteArrayEffect::CaseFolds,
    ByteArrayEffect::Encodes,
];

pub(in super::super) const STORAGE_TYPES: &[StorageType] =
    &[StorageType::Dict, StorageType::List, StorageType::Array];

pub(in super::super) const FORMAT_TYPES: &[FormatType] = &[
    FormatType::Sprintf,
    FormatType::Clock,
    FormatType::Binary,
    FormatType::Regsub,
];

/// The DSL word is the variant name, so each shape is a table entry rather
/// than a new parser.
pub(in super::super) const DEFAULT_FORM_FIRST_WORDS: &[DefaultFormFirstWord] = &[
    DefaultFormFirstWord::Integer,
    DefaultFormFirstWord::WindowPath,
    DefaultFormFirstWord::GridContent,
];

pub(in super::super) const PRESENTATIONS: &[ArgPresentation] =
    &[ArgPresentation::BlockScript, ArgPresentation::InlineScript];

pub(in super::super) const FRAME_LEVEL_WORDS: &[FrameLevelWord] = &[
    FrameLevelWord::None,
    FrameLevelWord::ArityParity,
    FrameLevelWord::Upvar,
    FrameLevelWord::LeadingProbe,
];

pub(in super::super) const FRAME_LAYOUTS: &[FrameArgLayout] = &[
    FrameArgLayout::AliasPairs,
    FrameArgLayout::ScriptInSelectedFrame,
    FrameArgLayout::ScriptInCurrentFrame,
    FrameArgLayout::OpaqueCallerVars,
];

pub(in super::super) const LOWERING_HOOKS: &[LoweringHookId] = &[
    LoweringHookId::Expr,
    LoweringHookId::Return,
    LoweringHookId::Set,
    LoweringHookId::Incr,
    LoweringHookId::AppendOrLappend,
    LoweringHookId::Unset,
    LoweringHookId::Global,
    LoweringHookId::Variable,
    LoweringHookId::Upvar,
    LoweringHookId::Proc,
    LoweringHookId::When,
    LoweringHookId::NamespaceEval,
    LoweringHookId::If,
    LoweringHookId::Switch,
    LoweringHookId::For,
    LoweringHookId::While,
    LoweringHookId::Foreach,
    LoweringHookId::Lmap,
    LoweringHookId::ForeachLine,
    LoweringHookId::Catch,
    LoweringHookId::Try,
    LoweringHookId::Dict,
    LoweringHookId::Eval,
    LoweringHookId::Uplevel,
    LoweringHookId::Apply,
    LoweringHookId::ArrayFor,
];

pub(in super::super) const CODEGEN_HOOKS: &[CodegenHookId] = &[
    CodegenHookId::Lassign,
    CodegenHookId::Llength,
    CodegenHookId::Lrange,
    CodegenHookId::Linsert,
    CodegenHookId::Lset,
    CodegenHookId::Dict,
    CodegenHookId::Array,
    CodegenHookId::Namespace,
    CodegenHookId::Append,
    CodegenHookId::Lappend,
    CodegenHookId::Unset,
    CodegenHookId::Tailcall,
    CodegenHookId::Concat,
    CodegenHookId::Global,
    CodegenHookId::Upvar,
    CodegenHookId::Uplevel,
    CodegenHookId::Set,
    CodegenHookId::ArraySet,
    CodegenHookId::ArrayUnset,
    CodegenHookId::Variable,
    CodegenHookId::NamespaceUpvar,
];

pub(in super::super) const INLINE_CODEGEN_HOOKS: &[InlineCodegenHookId] = &[
    InlineCodegenHookId::Expr,
    InlineCodegenHookId::Incr,
    InlineCodegenHookId::Set,
    InlineCodegenHookId::InfoExists,
    InlineCodegenHookId::InfoLevel,
    InlineCodegenHookId::InfoCommandsResolve,
    InlineCodegenHookId::NamespaceCurrent,
    InlineCodegenHookId::NamespaceOrigin,
    InlineCodegenHookId::NamespaceCode,
    InlineCodegenHookId::String,
    InlineCodegenHookId::Lindex,
    InlineCodegenHookId::Lrange,
    InlineCodegenHookId::Lreplace,
    InlineCodegenHookId::Linsert,
    InlineCodegenHookId::Regexp,
    InlineCodegenHookId::List,
    InlineCodegenHookId::Array,
    InlineCodegenHookId::DictGet,
    InlineCodegenHookId::Catch,
    InlineCodegenHookId::Return,
    InlineCodegenHookId::Error,
    InlineCodegenHookId::Break,
    InlineCodegenHookId::Continue,
    InlineCodegenHookId::Try,
    InlineCodegenHookId::Yield,
    InlineCodegenHookId::YieldTo,
    InlineCodegenHookId::StringIs,
    InlineCodegenHookId::StringMatch,
    InlineCodegenHookId::StringReplace,
    InlineCodegenHookId::StringTrim,
    InlineCodegenHookId::StringTrimLeft,
    InlineCodegenHookId::StringTrimRight,
];

pub(in super::super) const RETURN_TYPE_HOOKS: &[ReturnTypeHookId] = &[
    ReturnTypeHookId::Regexp,
    ReturnTypeHookId::Lsearch,
    ReturnTypeHookId::Regsub,
    ReturnTypeHookId::Scan,
    ReturnTypeHookId::Pid,
];

pub(in super::super) const ANALYSER_HOOKS: &[AnalyserHookId] = &[
    AnalyserHookId::Proc,
    AnalyserHookId::OptProc,
    AnalyserHookId::Apply,
    AnalyserHookId::Uplevel,
    AnalyserHookId::NamespaceEval,
    AnalyserHookId::NamespaceEnsemble,
    AnalyserHookId::NamespaceImport,
    AnalyserHookId::NamespaceExport,
    AnalyserHookId::NamespaceForget,
    AnalyserHookId::NamespacePath,
    AnalyserHookId::NamespaceUnknown,
    AnalyserHookId::Foreach,
    AnalyserHookId::Switch,
    AnalyserHookId::Catch,
    AnalyserHookId::DictWith,
    AnalyserHookId::InterpAlias,
    AnalyserHookId::InterpEval,
    AnalyserHookId::InterpCreate,
    AnalyserHookId::InterpDelete,
    AnalyserHookId::InterpHide,
    AnalyserHookId::InterpExpose,
    AnalyserHookId::Rename,
    AnalyserHookId::OoDefine,
    AnalyserHookId::OoObjdefine,
    AnalyserHookId::PackageRequire,
    AnalyserHookId::PackageProvide,
    AnalyserHookId::PackageIfneeded,
    AnalyserHookId::PackagePrefer,
    AnalyserHookId::Source,
    AnalyserHookId::RegexPatternCapture,
    AnalyserHookId::Load,
];

pub(super) const INTRINSICS: &[IntrinsicId] = &[
    IntrinsicId::ListAssign,
    IntrinsicId::ListLength,
    IntrinsicId::ListIndex,
    IntrinsicId::ListRange,
    IntrinsicId::ListReplace,
    IntrinsicId::ListInsert,
    IntrinsicId::ListSet,
    IntrinsicId::ListConstruct,
    IntrinsicId::DictGet,
    IntrinsicId::DictSet,
    IntrinsicId::DictUnset,
    IntrinsicId::DictIncr,
    IntrinsicId::DictAppend,
    IntrinsicId::DictListAppend,
    IntrinsicId::StringIndex,
    IntrinsicId::StringRange,
    IntrinsicId::StringEqual,
    IntrinsicId::StringCompare,
    IntrinsicId::StringReplace,
    IntrinsicId::StringLength,
    IntrinsicId::StringIs,
    IntrinsicId::Regexp,
    IntrinsicId::InfoExists,
    IntrinsicId::InfoLevel,
    IntrinsicId::InfoCommandsResolve,
    IntrinsicId::ArrayExists,
    IntrinsicId::ArrayNames,
    IntrinsicId::ArraySize,
    IntrinsicId::Concat,
    IntrinsicId::ChannelWrite,
    IntrinsicId::ProcedureNoOp,
    IntrinsicId::NamespaceCurrent,
    IntrinsicId::NamespaceOrigin,
    IntrinsicId::NamespaceCode,
];
