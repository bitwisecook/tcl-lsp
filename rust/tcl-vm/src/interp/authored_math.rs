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

//! Explicit authored fixed functions and their independent interpreter state.

use super::Vm;
use crate::{TclError, Value};
use std::{cell::Cell, rc::Rc};
use tcl_runtime_api::expression_policy::AuthoredMathFunctionProvider;

pub(super) struct AuthoredMathState {
    pub(super) seed: i64,
    precision: Rc<Cell<u8>>,
}

impl Default for AuthoredMathState {
    fn default() -> Self {
        Self {
            seed: tcl_syntax::expr::rand::seed_from_wide(1),
            precision: Rc::new(Cell::new(12)),
        }
    }
}

impl Vm {
    /// Install or remove an explicit authored Tcl84 fixed-function model.
    /// The parser and numeric providers must already be installed. Each actual
    /// interpreter owns its independent deterministic seed-one stream and
    /// precision-twelve formatter. No native table or compiler receipt is issued.
    #[must_use]
    pub fn set_logical_math_function_provider(
        &mut self,
        provider: Option<AuthoredMathFunctionProvider>,
    ) -> bool {
        if let Some(selected) = provider {
            let Some(mut policy) = self.expression_evaluation_policy() else {
                return false;
            };
            policy.authored_functions = Some(selected);
            if self.actual_engine_profile.is_none()
                || tcl_registry::authored_math_functions::provider(&policy).is_none()
            {
                return false;
            }
        }
        if self.logical_providers.math_functions == provider {
            return true;
        }
        self.advance_native_operation_context();
        self.logical_providers.math_functions = provider;
        self.authored_math = provider.map(|_| AuthoredMathState::default());
        self.bump_cmd_epoch();
        self.profile_generation = self.profile_generation.wrapping_add(1);
        self.eval_cache.clear();
        self.eval_cache_plain.clear();
        self.module_procs.clear();
        self.install_native_precision_trace();
        true
    }

    pub(crate) fn authored_math_provider(&self) -> Option<AuthoredMathFunctionProvider> {
        tcl_registry::authored_math_functions::provider(&self.expression_evaluation_policy()?)
    }

    pub(crate) fn authored_math_format(&self) -> Option<crate::value::DoubleFormatContext> {
        self.authored_math_provider()?;
        Some(crate::value::DoubleFormatContext::authored_tcl84(
            Rc::clone(&self.authored_math.as_ref()?.precision),
        ))
    }

    pub(crate) fn format_authored_math_result(&self, value: Value) -> Value {
        match self.authored_math_format() {
            Some(context) => value.with_authored_double_format(context),
            None => value,
        }
    }

    pub(crate) fn authored_math_random(
        &mut self,
        seed: Option<&Value>,
    ) -> tcl_runtime_api::Completion<Value> {
        use tcl_syntax::logical_numeric_simulation::LogicalNumericInputStage;
        if self.authored_math_provider().is_none() {
            return self.refuse_host_command("authored random provider is unavailable".into());
        }
        if let Err(refused) = self.confine_generator(if seed.is_some() { "srand" } else { "rand" })
        {
            return refused;
        }
        let next_seed = if let Some(value) = seed {
            let Some(simulation) = self
                .authored_numeric_context()
                .and_then(|context| context.simulation)
            else {
                return self
                    .refuse_host_command("authored random numeric policy is unavailable".into());
            };
            // C8.4 ExprSrandFunc keeps an operand's integer representation
            // (VerifyExprObjType), so the formatter's "-8" spelling of the
            // minimum wide never reaches the seed.
            let integer = value.integer_representation().or_else(|| {
                simulation
                    .parse_number(&value.string_bytes(), LogicalNumericInputStage::Integer)
                    .ok()
                    .and_then(|number| {
                        tcl_syntax::expr::wide::parsed_literal(
                            tcl_dialect::NativeArithmetic::Tcl84Wide,
                            &number,
                        )
                        .ok()
                    })
            });
            match integer {
                Some(seed) => Some(seed),
                None => {
                    return super::err(format!("expected integer but got \"{}\"", value.to_str()));
                }
            }
        } else {
            None
        };
        let state = self
            .authored_math
            .as_mut()
            .expect("installed authored state");
        if let Some(seed) = next_seed {
            state.seed = tcl_syntax::expr::rand::seed_from_wide(seed);
        }
        let value = Value::double(tcl_syntax::expr::rand::next_draw(&mut state.seed));
        super::ok(self.format_authored_math_result(value))
    }

    pub(super) fn prepare_authored_expression_bytes(
        &mut self,
        source: &[u8],
    ) -> Option<Result<tcl_syntax::expr::NativeExprNode, TclError>> {
        use tcl_registry::authored_math_functions::AuthoredFunctionPreparationError as Error;
        let policy = self.expression_evaluation_policy()?;
        tcl_registry::authored_math_functions::provider(&policy)?;
        Some(
            match tcl_registry::authored_math_functions::prepare(source, &policy) {
                Ok(tree) => Ok(tree),
                Err(Error::Rejected(diagnostic)) => Err(TclError::from_completion(
                    tcl_runtime_api::Completion::new_error_metadata(
                        tcl_runtime_api::Code::Error,
                        Value::from_string_bytes(diagnostic.message),
                        crate::command::options_dict(tcl_runtime_api::Code::Error, 0, &[]),
                    ),
                )),
                Err(Error::Unavailable) => Err(self.refuse_expression_bytes(
                    source,
                    tcl_runtime_api::NativeExpressionRefusal::FunctionDispatchPolicyUnavailable,
                )),
            },
        )
    }
}
