// SPDX-License-Identifier: AGPL-3.0-or-later
//! Allocation-only projections of original wrong-args and rewrite headers.

use super::{
    native_arguments::OriginalArguments, native_operation_currency::NativeOperationScope, Interp,
    TclObj,
};
use crate::obj::{self, NativeObjectLifetime};
use std::{
    ops::Range,
    rc::{Rc, Weak},
};
use tcl_cmd_core::CmdError;
use tcl_syntax::value::ValueError;

struct RewriteProjection {
    birth: Weak<()>,
    words: Range<usize>,
    source_length: usize,
    removed: usize,
    inserted: usize,
}

struct HandlerProjection {
    birth: Weak<()>,
    words: Range<usize>,
    removed: usize,
}

pub(super) struct OriginalArgumentUsage {
    originals: OriginalArguments,
    interpreter: Interp,
    arguments: Vec<*mut TclObj>,
    header_words: usize,
    rewrite: Option<RewriteProjection>,
    handler: Option<HandlerProjection>,
    selects_handler: bool,
}

impl OriginalArgumentUsage {
    pub(super) fn capture(
        interpreter: &Interp,
        arguments: &[*mut TclObj],
        header_words: usize,
        selects_handler: bool,
    ) -> Result<Self, CmdError> {
        let _scope =
            NativeOperationScope::enter(interpreter).map_err(CmdError::from_execution_refusal)?;
        // Projections borrow the actual installed roles. Header leases protect
        // allocations without cloning an Owned role or changing native sharing.
        let mut projected: Vec<_> = arguments
            .iter()
            .map(|&word| {
                obj::check_native_liveness(word)?;
                Ok(NativeObjectLifetime::retain(word))
            })
            .collect::<Result<Vec<_>, ValueError>>()?;
        let rewrite = interpreter
            .ensemble_rewrite
            .borrow()
            .as_ref()
            .map(|rewrite| -> Result<RewriteProjection, ValueError> {
                let start = projected.len();
                projected.extend(
                    rewrite
                        .source
                        .iter()
                        .take(rewrite.removed)
                        .map(|word| {
                            obj::check_native_liveness(word.as_ptr())?;
                            Ok(NativeObjectLifetime::retain(word.as_ptr()))
                        })
                        .collect::<Result<Vec<_>, ValueError>>()?,
                );
                Ok(RewriteProjection {
                    birth: Rc::downgrade(&rewrite.birth),
                    words: start..projected.len(),
                    source_length: rewrite.source.len(),
                    removed: rewrite.removed,
                    inserted: rewrite.inserted,
                })
            })
            .transpose()?;
        let handler = if selects_handler {
            interpreter
                .handler_usage_adapters
                .borrow()
                .iter()
                .rev()
                .find(|handler| arguments.starts_with(&handler.parse_prefix))
                .map(|handler| -> Result<HandlerProjection, ValueError> {
                    let start = projected.len();
                    projected.extend(
                        handler
                            .original_prefix
                            .iter()
                            .map(|word| {
                                obj::check_native_liveness(word.as_ptr())?;
                                Ok(NativeObjectLifetime::retain(word.as_ptr()))
                            })
                            .collect::<Result<Vec<_>, ValueError>>()?,
                    );
                    Ok(HandlerProjection {
                        birth: Rc::downgrade(&handler.birth),
                        words: start..projected.len(),
                        removed: handler.parse_prefix.len(),
                    })
                })
                .transpose()?
        } else {
            None
        };
        let pointers: Vec<_> = projected.iter().map(NativeObjectLifetime::as_ptr).collect();
        let originals = OriginalArguments::enter(interpreter, &pointers)?;
        drop(projected);
        let result = Self {
            originals,
            interpreter: interpreter.clone(),
            arguments: arguments.to_vec(),
            header_words: header_words.min(arguments.len()),
            rewrite,
            handler,
            selects_handler,
        };
        result.check()?;
        Ok(result)
    }

    pub(super) fn check(&self) -> Result<(), CmdError> {
        self.originals.check()?;
        let same_rewrite = match (
            &self.rewrite,
            self.interpreter.ensemble_rewrite.borrow().as_ref(),
        ) {
            (None, None) => true,
            (Some(original), Some(current)) => {
                Weak::ptr_eq(&original.birth, &Rc::downgrade(&current.birth))
            }
            _ => false,
        };
        if !same_rewrite {
            return Err(self.refuse("original argument usage rewrite replaced"));
        }
        if self.selects_handler {
            let same_handler = {
                let handlers = self.interpreter.handler_usage_adapters.borrow();
                let current = handlers
                    .iter()
                    .rev()
                    .find(|handler| self.arguments.starts_with(&handler.parse_prefix));
                match (&self.handler, current) {
                    (None, None) => true,
                    (Some(original), Some(current)) => {
                        Weak::ptr_eq(&original.birth, &Rc::downgrade(&current.birth))
                    }
                    _ => false,
                }
            };
            if !same_handler {
                return Err(self.refuse("original argument usage adapter replaced"));
            }
        }
        Ok(())
    }

    fn refuse(&self, purpose: &'static str) -> CmdError {
        let cause = self
            .interpreter
            .clone()
            .refuse_completion_value_access(ValueError::CommandProtocolUnavailable(purpose));
        CmdError::from_execution_refusal(cause)
    }

    fn index_word(
        &self,
        interpreter: &mut Interp,
        index: usize,
    ) -> Result<(Rc<[u8]>, bool), CmdError> {
        self.originals.check_for(interpreter)?;
        self.check()?;
        let original = self.originals.original(index)?;
        self.originals
            .check_original_for(interpreter, index, original)?;
        let outcome = interpreter.native_index_usage_bytes(original);
        self.originals
            .check_original_for(interpreter, index, original)?;
        self.check()?;
        outcome.map_err(CmdError::from)
    }

    pub(super) fn render_header(
        &self,
        interpreter: &mut Interp,
        protocol: tcl_registry::native_usage::NativeUsageProtocol,
    ) -> Result<Vec<u8>, CmdError> {
        self.originals.check_for(interpreter)?;
        let mut header = Vec::new();
        for index in 0..self.header_words {
            let (bytes, cached) = self.index_word(interpreter, index)?;
            header.push((bytes, cached, false));
        }
        let mut rewrites = Vec::new();
        if let Some(rewrite) = &self.rewrite {
            let mut original_prefix = Vec::new();
            for index in rewrite.words.clone() {
                let (bytes, cached) = self.index_word(interpreter, index)?;
                original_prefix.push((bytes, cached, true));
            }
            rewrites.push(tcl_cmd_core::ensemble::ArgumentUsageRewrite {
                original_prefix,
                removed_words: rewrite.inserted,
            });
        }
        if let Some(handler) = &self.handler {
            if self.header_words < handler.removed {
                rewrites.clear();
            } else {
                let mut original_prefix = Vec::new();
                for index in handler.words.clone() {
                    let (bytes, cached) = self.index_word(interpreter, index)?;
                    original_prefix.push((bytes, cached, true));
                }
                rewrites.push(tcl_cmd_core::ensemble::ArgumentUsageRewrite {
                    original_prefix,
                    removed_words: handler.removed,
                });
            }
        }
        self.check()?;
        let header = tcl_cmd_core::ensemble::rewrite_argument_usage(&header, &rewrites);
        let words = header
            .iter()
            .map(|(bytes, cached, rewritten)| {
                if !*cached {
                    tcl_registry::native_usage::NativeUsageWord::Original(bytes)
                } else if *rewritten {
                    tcl_registry::native_usage::NativeUsageWord::RewrittenIndex(bytes)
                } else {
                    tcl_registry::native_usage::NativeUsageWord::CanonicalIndex(bytes)
                }
            })
            .collect::<Vec<_>>();
        protocol
            .render_header(&words)
            .ok_or_else(|| self.refuse("argument usage"))
    }

    pub(super) fn procedure_prefix(
        &self,
        supplied: usize,
        parameters: usize,
    ) -> Result<Option<(Vec<Rc<[u8]>>, usize)>, CmdError> {
        self.check()?;
        let Some(rewrite) = &self.rewrite else {
            return Ok(None);
        };
        let user_arguments = rewrite.source_length.saturating_sub(rewrite.removed);
        let drop_parameters = supplied.saturating_sub(user_arguments);
        if drop_parameters > parameters {
            return Ok(None);
        }
        let mut words = Vec::new();
        for index in rewrite.words.clone() {
            self.check()?;
            let outcome = self.originals.bytes(index);
            self.check()?;
            words.push(outcome?);
        }
        Ok(Some((words, drop_parameters)))
    }
}

#[cfg(test)]
mod tests;
