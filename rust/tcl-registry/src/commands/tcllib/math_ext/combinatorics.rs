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

//! Counting: `math::combinatorics` and the figurate numbers of `math::figurate`.

use super::{Arity, Row};

/// The `math::combinatorics` package.
pub(super) const MATH__COMBINATORICS_CMDS: &[Row] = &[
    (
        "math::combinatorics::permutations",
        Arity::exact(1),
        &["math::combinatorics::permutations n"],
        "Return the number of permutations of n items.",
    ),
    (
        "math::combinatorics::variations",
        Arity::exact(2),
        &["math::combinatorics::variations n k"],
        "Return the number of variations k items selected from the total of n items.",
    ),
    (
        "math::combinatorics::combinations",
        Arity::exact(2),
        &["math::combinatorics::combinations n k"],
        "Return the number of combinations of k items selected from the total of n items.",
    ),
    (
        "math::combinatorics::derangements",
        Arity::exact(1),
        &["math::combinatorics::derangements n"],
        "Return the number of derangements of n items.",
    ),
    (
        "math::combinatorics::catalan",
        Arity::exact(1),
        &["math::combinatorics::catalan n"],
        "Return the n'th Catalan number.",
    ),
    (
        "math::combinatorics::firstStirling",
        Arity::exact(2),
        &["math::combinatorics::firstStirling n m"],
        "Calculate a Stirling number of the first kind (signed version, m cycles in a permutation of n items) Number of it",
    ),
    (
        "math::combinatorics::secondStirling",
        Arity::exact(2),
        &["math::combinatorics::secondStirling n m"],
        "Calculate a Stirling number of the second kind (m non-empty subsets from n items) Number of items Number of subse",
    ),
    (
        "math::combinatorics::partitionP",
        Arity::exact(1),
        &["math::combinatorics::partitionP n"],
        "Calculate the number of ways an integer n can be written as the sum of positive integers.",
    ),
];

/// The `math::figurate` package.
pub(super) const MATH__FIGURATE_CMDS: &[Row] = &[
    (
        "math::figurate::sum_sequence",
        Arity::exact(1),
        &["math::figurate::sum_sequence n"],
        "Return the sum of integers 1, 2, ..., n.",
    ),
    (
        "math::figurate::sum_squares",
        Arity::exact(1),
        &["math::figurate::sum_squares n"],
        "Return the sum of squares 1**2, 2**2, ..., n**2.",
    ),
    (
        "math::figurate::sum_cubes",
        Arity::exact(1),
        &["math::figurate::sum_cubes n"],
        "Return the sum of cubes 1**3, 2**3, ..., n**3.",
    ),
    (
        "math::figurate::sum_4th_power",
        Arity::exact(1),
        &["math::figurate::sum_4th_power n"],
        "Return the sum of 4th powers 1**4, 2**4, ..., n**4.",
    ),
    (
        "math::figurate::sum_5th_power",
        Arity::exact(1),
        &["math::figurate::sum_5th_power n"],
        "Return the sum of 5th powers 1**5, 2**5, ..., n**5.",
    ),
    (
        "math::figurate::sum_6th_power",
        Arity::exact(1),
        &["math::figurate::sum_6th_power n"],
        "Return the sum of 6th powers 1**6, 2**6, ..., n**6.",
    ),
    (
        "math::figurate::sum_7th_power",
        Arity::exact(1),
        &["math::figurate::sum_7th_power n"],
        "Return the sum of 7th powers 1**7, 2**7, ..., n**7.",
    ),
    (
        "math::figurate::sum_8th_power",
        Arity::exact(1),
        &["math::figurate::sum_8th_power n"],
        "Return the sum of 8th powers 1**8, 2**8, ..., n**8.",
    ),
    (
        "math::figurate::sum_9th_power",
        Arity::exact(1),
        &["math::figurate::sum_9th_power n"],
        "Return the sum of 9th powers 1**9, 2**9, ..., n**9.",
    ),
    (
        "math::figurate::sum_10th_power",
        Arity::exact(1),
        &["math::figurate::sum_10th_power n"],
        "Return the sum of 10th powers 1**10, 2**10, ..., n**10.",
    ),
    (
        "math::figurate::sum_sequence_odd",
        Arity::exact(1),
        &["math::figurate::sum_sequence_odd n"],
        "Return the sum of odd integers 1, 3, ..., 2n-1 Highest integer in the sum.",
    ),
    (
        "math::figurate::sum_squares_odd",
        Arity::exact(1),
        &["math::figurate::sum_squares_odd n"],
        "Return the sum of odd squares 1**2, 3**2, ..., (2n-1)**2.",
    ),
    (
        "math::figurate::sum_cubes_odd",
        Arity::exact(1),
        &["math::figurate::sum_cubes_odd n"],
        "Return the sum of odd cubes 1**3, 3**3, ..., (2n-1)**3.",
    ),
    (
        "math::figurate::sum_4th_power_odd",
        Arity::exact(1),
        &["math::figurate::sum_4th_power_odd n"],
        "Return the sum of odd 4th powers 1**4, 2**4, ..., (2n-1)**4.",
    ),
    (
        "math::figurate::sum_5th_power_odd",
        Arity::exact(1),
        &["math::figurate::sum_5th_power_odd n"],
        "Return the sum of odd 5th powers 1**5, 2**5, ..., (2n-1)**5.",
    ),
    (
        "math::figurate::sum_6th_power_odd",
        Arity::exact(1),
        &["math::figurate::sum_6th_power_odd n"],
        "Return the sum of odd 6th powers 1**6, 2**6, ..., (2n-1)**6.",
    ),
    (
        "math::figurate::sum_7th_power_odd",
        Arity::exact(1),
        &["math::figurate::sum_7th_power_odd n"],
        "Return the sum of odd 7th powers 1**7, 2**7, ..., (2n-1)**7.",
    ),
    (
        "math::figurate::sum_8th_power_odd",
        Arity::exact(1),
        &["math::figurate::sum_8th_power_odd n"],
        "Return the sum of odd 8th powers 1**8, 2**8, ..., (2n-1)**8.",
    ),
    (
        "math::figurate::sum_9th_power_odd",
        Arity::exact(1),
        &["math::figurate::sum_9th_power_odd n"],
        "Return the sum of odd 9th powers 1**9, 2**9, ..., (2n-1)**9.",
    ),
    (
        "math::figurate::sum_10th_power_odd",
        Arity::exact(1),
        &["math::figurate::sum_10th_power_odd n"],
        "Return the sum of odd 10th powers 1**10, 2**10, ..., (2n-1)**10.",
    ),
    (
        "math::figurate::oblong",
        Arity::exact(1),
        &["math::figurate::oblong n"],
        "Return the nth oblong number (twice the nth triangular number) Required index.",
    ),
    (
        "math::figurate::pronic",
        Arity::exact(1),
        &["math::figurate::pronic n"],
        "Return the nth pronic number (synonym for oblong) Required index.",
    ),
    (
        "math::figurate::triangular",
        Arity::exact(1),
        &["math::figurate::triangular n"],
        "Return the nth triangular number Required index.",
    ),
    (
        "math::figurate::square",
        Arity::exact(1),
        &["math::figurate::square n"],
        "Return the nth square number Required index.",
    ),
    (
        "math::figurate::cubic",
        Arity::exact(1),
        &["math::figurate::cubic n"],
        "Return the nth cubic number Required index.",
    ),
    (
        "math::figurate::biquadratic",
        Arity::exact(1),
        &["math::figurate::biquadratic n"],
        "Return the nth biquaratic number (i.e.",
    ),
    (
        "math::figurate::centeredTriangular",
        Arity::exact(1),
        &["math::figurate::centeredTriangular n"],
        "Return the nth centered triangular number (items arranged in concentric squares) Required index.",
    ),
    (
        "math::figurate::centeredSquare",
        Arity::exact(1),
        &["math::figurate::centeredSquare n"],
        "Return the nth centered square number (items arranged in concentric squares) Required index.",
    ),
    (
        "math::figurate::centeredPentagonal",
        Arity::exact(1),
        &["math::figurate::centeredPentagonal n"],
        "Return the nth centered pentagonal number (items arranged in concentric pentagons) Required index.",
    ),
    (
        "math::figurate::centeredHexagonal",
        Arity::exact(1),
        &["math::figurate::centeredHexagonal n"],
        "Return the nth centered hexagonal number (items arranged in concentric hexagons) Required index.",
    ),
    (
        "math::figurate::centeredCube",
        Arity::exact(1),
        &["math::figurate::centeredCube n"],
        "Return the nth centered cube number (items arranged in concentric cubes) Required index.",
    ),
    (
        "math::figurate::decagonal",
        Arity::exact(1),
        &["math::figurate::decagonal n"],
        "Return the nth decagonal number (items arranged in decagons with one common vertex) Required index.",
    ),
    (
        "math::figurate::heptagonal",
        Arity::exact(1),
        &["math::figurate::heptagonal n"],
        "Return the nth heptagonal number (items arranged in heptagons with one common vertex) Required index.",
    ),
    (
        "math::figurate::hexagonal",
        Arity::exact(1),
        &["math::figurate::hexagonal n"],
        "Return the nth hexagonal number (items arranged in hexagons with one common vertex) Required index.",
    ),
    (
        "math::figurate::octagonal",
        Arity::exact(1),
        &["math::figurate::octagonal n"],
        "Return the nth octagonal number (items arranged in octagons with one common vertex) Required index.",
    ),
    (
        "math::figurate::octahedral",
        Arity::exact(1),
        &["math::figurate::octahedral n"],
        "Return the nth octahedral number (items arranged in octahedrons with a common centre) Required index.",
    ),
    (
        "math::figurate::pentagonal",
        Arity::exact(1),
        &["math::figurate::pentagonal n"],
        "Return the nth pentagonal number (items arranged in pentagons with one common vertex) Required index.",
    ),
    (
        "math::figurate::squarePyramidral",
        Arity::exact(1),
        &["math::figurate::squarePyramidral n"],
        "Return the nth square pyramidral number (items arranged in a square pyramid) Required index.",
    ),
    (
        "math::figurate::tetrahedral",
        Arity::exact(1),
        &["math::figurate::tetrahedral n"],
        "Return the nth tetrahedral number (items arranged in a triangular pyramid) Required index.",
    ),
    (
        "math::figurate::pentatope",
        Arity::exact(1),
        &["math::figurate::pentatope n"],
        "Return the nth pentatope number (items arranged in the four-dimensional analogue of a triangular pyramid) Require",
    ),
];
