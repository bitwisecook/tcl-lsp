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

//! tcllib `math::*` sub-package commands.
//!
//! Numeric sub-packages (geometry, linearalgebra, bignum, special, trig, calculus, decimal, figurate, …), each attributed to its namespace package.

use crate::prelude::*;

mod algebra;
mod analysis;
mod combinatorics;
mod geometry;
mod number_systems;

use algebra::{MATH__LINEARALGEBRA_CMDS, MATH__POLYNOMIALS_CMDS, MATH__RATIONALFUNCTIONS_CMDS};
use analysis::{
    MATH__CALCULUS_CMDS, MATH__FILTERS_CMDS, MATH__FOURIER_CMDS, MATH__INTERPOLATE_CMDS,
    MATH__OPTIMIZE_CMDS, MATH__SPECIAL_CMDS, MATH__TRIG_CMDS,
};
use combinatorics::{MATH__COMBINATORICS_CMDS, MATH__FIGURATE_CMDS};
use geometry::MATH__GEOMETRY_CMDS;
use number_systems::{
    MATH__BIGNUM_CMDS, MATH__COMPLEXNUMBERS_CMDS, MATH__DECIMAL_CMDS, MATH__EXACT_CMDS,
};

/// A row in a package command table: name, arity, synopsis, and summary.
type Row = (&'static str, Arity, &'static [&'static str], &'static str);

/// Build command specs for a package from its table.
fn rows(pkg: &'static str, table: &'static [Row]) -> Vec<CommandSpec> {
    table
        .iter()
        .map(|&(name, arity, synopsis, summary)| CommandSpec {
            name,
            arity,
            hover: Some(HoverSnippet::brief(summary, synopsis, "tcllib package")),
            tcllib_package: Some(pkg),
            required_package: Some(pkg),
            ..CommandSpec::DEFAULT
        })
        .collect()
}

/// The `math` package.
const MATH_CMDS: &[Row] = &[
    (
        "math::ln_Gamma",
        Arity::exact(1),
        &["math::ln_Gamma z"],
        "Returns the natural logarithm of the Gamma function for the argument z.",
    ),
    (
        "math::factorial",
        Arity::exact(1),
        &["math::factorial x"],
        "Returns the factorial of the argument x.",
    ),
    (
        "math::choose",
        Arity::exact(1),
        &["math::choose n k"],
        "Returns the binomial coefficient C(n, k) [example { C(n,k) = n!.",
    ),
    (
        "math::Beta",
        Arity::exact(1),
        &["math::Beta z w"],
        "Returns the Beta function of the parameters z and w.",
    ),
];

/// The `math::PCA` package.
const MATH__PCA_CMDS: &[Row] = &[(
    "math::PCA::createPCA",
    Arity::at_least(1),
    &["math::PCA::createPCA data args"],
    "Create a new object, based on the data that are passed via the data argument.",
)];

/// The `math::probopt` package.
const MATH__PROBOPT_CMDS: &[Row] = &[
    (
        "math::probopt::pso",
        Arity::at_least(3),
        &["math::probopt::pso function bounds args"],
        "The particle swarm optimisation algorithm uses the idea that the candidate optimum points should swarm around the",
    ),
    (
        "math::probopt::sce",
        Arity::at_least(3),
        &["math::probopt::sce function bounds args"],
        "The shuffled complex evolution algorithm is an extension of the Nelder-Mead algorithm that uses multiple complexe",
    ),
    (
        "math::probopt::diffev",
        Arity::at_least(3),
        &["math::probopt::diffev function bounds args"],
        "The differential evolution algorithm uses a number of initial points that are then updated using randomly selecte",
    ),
    (
        "math::probopt::lipoMax",
        Arity::at_least(3),
        &["math::probopt::lipoMax function bounds args"],
        "The Lipschitz optimisation algorithm uses the Lipschitz property of the given function to find a maximum in the g",
    ),
    (
        "math::probopt::adaLipoMax",
        Arity::at_least(3),
        &["math::probopt::adaLipoMax function bounds args"],
        "The adaptive Lipschitz optimisation algorithm uses the Lipschitz property of the given function to find a maximum",
    ),
];

/// The `math::statistics` package.
const MATH__STATISTICS_CMDS: &[Row] = &[
    (
        "math::statistics::bootstrap",
        Arity::at_least(2),
        &["math::statistics::bootstrap data sampleSize numberSamples"],
        "Create a subsample or subsamples from a given list of data.",
    ),
    (
        "math::statistics::tstat",
        Arity::at_least(1),
        &["math::statistics::tstat dof alpha"],
        "Returns the value of the t-distribution t* satisfying [example { P(t*) = 1 - alpha/2 P(-t*) = alpha/2 }] for the ",
    ),
    (
        "math::statistics::incompleteGamma",
        Arity::at_least(2),
        &["math::statistics::incompleteGamma x p tol"],
        "Evaluate the incomplete Gamma integral [example { 1 / x p-1 P(p,x) = -------- | dt exp(-t) * t Gamma(p) / 0 }] - ",
    ),
    (
        "math::statistics::incompleteBeta",
        Arity::at_least(3),
        &["math::statistics::incompleteBeta a b x tol"],
        "Evaluate the incomplete Beta integral - First shape parameter - Second shape parameter - Value of x (limit of the",
    ),
    (
        "math::statistics::subdivide",
        Arity::exact(0),
        &["math::statistics::subdivide"],
        "Routine PM - not implemented yet.",
    ),
];

/// (package, command-table) pairs.
const GROUPS: &[(&str, &[Row])] = &[
    ("math", MATH_CMDS),
    ("math::PCA", MATH__PCA_CMDS),
    ("math::bignum", MATH__BIGNUM_CMDS),
    ("math::calculus", MATH__CALCULUS_CMDS),
    ("math::combinatorics", MATH__COMBINATORICS_CMDS),
    ("math::complexnumbers", MATH__COMPLEXNUMBERS_CMDS),
    ("math::decimal", MATH__DECIMAL_CMDS),
    ("math::exact", MATH__EXACT_CMDS),
    ("math::figurate", MATH__FIGURATE_CMDS),
    ("math::filters", MATH__FILTERS_CMDS),
    ("math::fourier", MATH__FOURIER_CMDS),
    ("math::geometry", MATH__GEOMETRY_CMDS),
    ("math::interpolate", MATH__INTERPOLATE_CMDS),
    ("math::linearalgebra", MATH__LINEARALGEBRA_CMDS),
    ("math::optimize", MATH__OPTIMIZE_CMDS),
    ("math::polynomials", MATH__POLYNOMIALS_CMDS),
    ("math::probopt", MATH__PROBOPT_CMDS),
    ("math::rationalfunctions", MATH__RATIONALFUNCTIONS_CMDS),
    ("math::special", MATH__SPECIAL_CMDS),
    ("math::statistics", MATH__STATISTICS_CMDS),
    ("math::trig", MATH__TRIG_CMDS),
];

/// Command-prefix callback positions for the numerical packages.  The flat
/// [`Row`] tables above cannot carry a `command_prefixes` field, so these are
/// applied by name in [`specs`].  Index is relative to the args after the
/// command name (these are full command names, not sub-commands).
///
/// Ground truth: the `math::calculus`, `math::optimize`, and `math::probopt`
/// man pages, each of which pins the `func` argument's signature (a procedure
/// taking a fixed number of arguments) — so these are `Exactly(n)`, unlike the
/// variadic/ambiguous callbacks elsewhere that stay `Unknown`.
const PREFIX_OVERRIDES: &[(&str, &[(u8, AppendedArity)])] = &[
    // `integral begin end nosteps func` — func(x).
    (
        "math::calculus::integral",
        &[(3, AppendedArity::Exactly(1))],
    ),
    // `integral2D xinterval yinterval func` — func(x y).
    (
        "math::calculus::integral2D",
        &[(2, AppendedArity::Exactly(2))],
    ),
    (
        "math::calculus::integral2D_accurate",
        &[(2, AppendedArity::Exactly(2))],
    ),
    // `integral3D xinterval yinterval zinterval func` — func(x y z).
    (
        "math::calculus::integral3D",
        &[(3, AppendedArity::Exactly(3))],
    ),
    (
        "math::calculus::integral3D_accurate",
        &[(3, AppendedArity::Exactly(3))],
    ),
    // `qk15 xstart xend func nosteps` — func(x).
    ("math::calculus::qk15", &[(2, AppendedArity::Exactly(1))]),
    (
        "math::calculus::qk15_detailed",
        &[(2, AppendedArity::Exactly(1))],
    ),
    // ODE steppers `… t tstep xvec func` — func(t xvec) (xvec is one list).
    (
        "math::calculus::eulerStep",
        &[(3, AppendedArity::Exactly(2))],
    ),
    (
        "math::calculus::heunStep",
        &[(3, AppendedArity::Exactly(2))],
    ),
    (
        "math::calculus::rungeKuttaStep",
        &[(3, AppendedArity::Exactly(2))],
    ),
    // `boundaryValueSecondOrder coeff_func force_func …` — two prefixes, each f(x).
    (
        "math::calculus::boundaryValueSecondOrder",
        &[
            (0, AppendedArity::Exactly(1)),
            (1, AppendedArity::Exactly(1)),
        ],
    ),
    // `newtonRaphson func deriv initval` — func(x) and deriv(x).
    (
        "math::calculus::newtonRaphson",
        &[
            (0, AppendedArity::Exactly(1)),
            (1, AppendedArity::Exactly(1)),
        ],
    ),
    // Root finders `f xb xe eps` — f(x).
    (
        "math::calculus::regula_falsi",
        &[(0, AppendedArity::Exactly(1))],
    ),
    (
        "math::calculus::root_bisection",
        &[(0, AppendedArity::Exactly(1))],
    ),
    (
        "math::calculus::root_secant",
        &[(0, AppendedArity::Exactly(1))],
    ),
    (
        "math::calculus::root_brent",
        &[(0, AppendedArity::Exactly(1))],
    ),
    (
        "math::calculus::root_chandrupatla",
        &[(0, AppendedArity::Exactly(1))],
    ),
    // `math::optimize::{minimum,maximum} begin end func maxerr` — func(x).
    ("math::optimize::minimum", &[(2, AppendedArity::Exactly(1))]),
    ("math::optimize::maximum", &[(2, AppendedArity::Exactly(1))]),
    // Probabilistic optimisers `function bounds args` — objective(coordVec) as
    // one list argument.
    ("math::probopt::pso", &[(0, AppendedArity::Exactly(1))]),
    ("math::probopt::sce", &[(0, AppendedArity::Exactly(1))]),
    ("math::probopt::diffev", &[(0, AppendedArity::Exactly(1))]),
    ("math::probopt::lipoMax", &[(0, AppendedArity::Exactly(1))]),
    (
        "math::probopt::adaLipoMax",
        &[(0, AppendedArity::Exactly(1))],
    ),
];

/// All command specs in this group.
pub fn specs() -> Vec<CommandSpec> {
    let mut out: Vec<CommandSpec> = GROUPS
        .iter()
        .flat_map(|&(pkg, table)| rows(pkg, table))
        .collect();
    // Patch in the callback positions the flat `Row` tables can't express.
    for spec in &mut out {
        if let Some(&(_, prefixes)) = PREFIX_OVERRIDES.iter().find(|&&(n, _)| n == spec.name) {
            spec.command_prefixes = prefixes;
        }
    }
    out
}
