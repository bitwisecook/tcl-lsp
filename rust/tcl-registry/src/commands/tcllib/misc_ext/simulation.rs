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

//! The `simulation::annealing`, `simulation::montecarlo` and `simulation::random` packages.

use super::{Arity, Row};

/// The `simulation::annealing` package.
pub(super) const SIMULATION__ANNEALING_CMDS: &[Row] = &[
    (
        "simulation::annealing::getOption",
        Arity::exact(1),
        &["simulation::annealing::getOption keyword"],
        "Get the value of an option given as part of the findMinimum command.",
    ),
    (
        "simulation::annealing::hasOption",
        Arity::exact(1),
        &["simulation::annealing::hasOption keyword"],
        "Returns 1 if the option is available, 0 if not.",
    ),
    (
        "simulation::annealing::setOption",
        Arity::exact(2),
        &["simulation::annealing::setOption keyword value"],
        "Set the value of the given option.",
    ),
    (
        "simulation::annealing::findMinimum",
        Arity::at_least(1),
        &["simulation::annealing::findMinimum args"],
        "Find the minimum of a function using simulated annealing.",
    ),
    (
        "simulation::annealing::findCombinatorialMinimum",
        Arity::at_least(1),
        &["simulation::annealing::findCombinatorialMinimum args"],
        "Find the minimum of a function of discrete variables using simulated annealing.",
    ),
];

/// The `simulation::montecarlo` package.
pub(super) const SIMULATION__MONTECARLO_CMDS: &[Row] = &[
    (
        "simulation::montecarlo::getOption",
        Arity::exact(1),
        &["simulation::montecarlo::getOption keyword"],
        "Get the value of an option given as part of the singleExperiment command.",
    ),
    (
        "simulation::montecarlo::hasOption",
        Arity::exact(1),
        &["simulation::montecarlo::hasOption keyword"],
        "Returns 1 if the option is available, 0 if not.",
    ),
    (
        "simulation::montecarlo::setOption",
        Arity::exact(2),
        &["simulation::montecarlo::setOption keyword value"],
        "Set the value of the given option.",
    ),
    (
        "simulation::montecarlo::setTrialResult",
        Arity::exact(1),
        &["simulation::montecarlo::setTrialResult values"],
        "Store the results of the trial for later analysis List of values to be stored.",
    ),
    (
        "simulation::montecarlo::setExpResult",
        Arity::exact(1),
        &["simulation::montecarlo::setExpResult values"],
        "Set the results of the entire experiment (typically used in the final phase).",
    ),
    (
        "simulation::montecarlo::getTrialResults",
        Arity::exact(0),
        &["simulation::montecarlo::getTrialResults"],
        "Get the results of all individual trials for analysis (typically used in the final phase or after completion of t",
    ),
    (
        "simulation::montecarlo::getExpResult",
        Arity::exact(0),
        &["simulation::montecarlo::getExpResult"],
        "Get the results of the entire experiment (typically used in the final phase or even after completion of the singl",
    ),
    (
        "simulation::montecarlo::transposeData",
        Arity::exact(1),
        &["simulation::montecarlo::transposeData values"],
        "Interchange columns and rows of a list of lists and return the result.",
    ),
    (
        "simulation::montecarlo::integral2D",
        Arity::at_least(1),
        &["simulation::montecarlo::integral2D ..."],
        "Integrate a function over a two-dimensional region using a Monte Carlo approach.",
    ),
    (
        "simulation::montecarlo::singleExperiment",
        Arity::at_least(1),
        &["simulation::montecarlo::singleExperiment args"],
        "Iterate code over a number of trials and store the results.",
    ),
];

/// The `simulation::random` package.
pub(super) const SIMULATION__RANDOM_CMDS: &[Row] = &[
    (
        "simulation::random::prng_Bernoulli",
        Arity::exact(1),
        &["simulation::random::prng_Bernoulli p"],
        "Create a command (PRNG) that generates numbers with a Bernoulli distribution: the value is either 1 or 0, with a ",
    ),
    (
        "simulation::random::prng_Discrete",
        Arity::exact(1),
        &["simulation::random::prng_Discrete n"],
        "Create a command (PRNG) that generates numbers 0 to n-1 with equal probability.",
    ),
    (
        "simulation::random::prng_Poisson",
        Arity::exact(1),
        &["simulation::random::prng_Poisson lambda"],
        "Create a command (PRNG) that generates numbers according to the Poisson distribution.",
    ),
    (
        "simulation::random::prng_Uniform",
        Arity::exact(2),
        &["simulation::random::prng_Uniform min max"],
        "Create a command (PRNG) that generates uniformly distributed numbers between min and max.",
    ),
    (
        "simulation::random::prng_Triangular",
        Arity::exact(2),
        &["simulation::random::prng_Triangular min max"],
        "Create a command (PRNG) that generates triangularly distributed numbers between min and max.",
    ),
    (
        "simulation::random::prng_SymmTriangular",
        Arity::exact(2),
        &["simulation::random::prng_SymmTriangular min max"],
        "Create a command (PRNG) that generates numbers distributed according to a symmetric triangle around the mean of m",
    ),
    (
        "simulation::random::prng_Exponential",
        Arity::exact(2),
        &["simulation::random::prng_Exponential min mean"],
        "Create a command (PRNG) that generates exponentially distributed numbers with a given minimum value and a given m",
    ),
    (
        "simulation::random::prng_Normal",
        Arity::exact(2),
        &["simulation::random::prng_Normal mean stdev"],
        "Create a command (PRNG) that generates normally distributed numbers with a given mean value and a given standard ",
    ),
    (
        "simulation::random::prng_Pareto",
        Arity::exact(2),
        &["simulation::random::prng_Pareto min steep"],
        "Create a command (PRNG) that generates numbers distributed according to Pareto with a given minimum value and a g",
    ),
    (
        "simulation::random::prng_Gumbel",
        Arity::exact(2),
        &["simulation::random::prng_Gumbel min f"],
        "Create a command (PRNG) that generates numbers distributed according to Gumbel with a given minimum value and a g",
    ),
    (
        "simulation::random::prng_chiSquared",
        Arity::exact(1),
        &["simulation::random::prng_chiSquared df"],
        "Create a command (PRNG) that generates numbers distributed according to the chi-squared distribution with df degr",
    ),
    (
        "simulation::random::prng_Disk",
        Arity::exact(1),
        &["simulation::random::prng_Disk rad"],
        "Create a command (PRNG) that generates (x,y)-coordinates for points uniformly spread over a disk of given radius.",
    ),
    (
        "simulation::random::prng_Sphere",
        Arity::exact(1),
        &["simulation::random::prng_Sphere rad"],
        "Create a command (PRNG) that generates (x,y,z)-coordinates for points uniformly spread over the surface of a sphe",
    ),
    (
        "simulation::random::prng_Ball",
        Arity::exact(1),
        &["simulation::random::prng_Ball rad"],
        "Create a command (PRNG) that generates (x,y,z)-coordinates for points uniformly spread within a ball of given rad",
    ),
    (
        "simulation::random::prng_Rectangle",
        Arity::exact(2),
        &["simulation::random::prng_Rectangle length width"],
        "Create a command (PRNG) that generates (x,y)-coordinates for points uniformly spread over a rectangle.",
    ),
    (
        "simulation::random::prng_Block",
        Arity::exact(3),
        &["simulation::random::prng_Block length width depth"],
        "Create a command (PRNG) that generates (x,y,z)-coordinates for points uniformly spread over a block Length of the",
    ),
];
