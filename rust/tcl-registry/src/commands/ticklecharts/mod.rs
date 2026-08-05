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

//! The `ticklecharts` package — a `TclOO` wrapper around Apache `ECharts` by
//! nico-robert (<https://github.com/nico-robert/ticklecharts>).
//!
//! The public entry points are `oo::class`es (`ticklecharts::chart`,
//! `ticklecharts::chart3D`, `ticklecharts::timeline`, `ticklecharts::Gridlayout`,
//! …) instantiated with `new`; the returned object handle is then dispatched as
//! `$chart Xaxis -name … -type value …`.  Each such method forwards its
//! `-option value` pairs to a namespace proc that declares them via
//! ticklecharts' `setdef` DSL, so the option surface is static and enumerable
//! from source — but very large (hundreds of options across ~27 series types).
//!
//! This pack models the class factories via [`ObjectClassSpec`] and the chart
//! object's methods with their most-used options, so `$chart Xaxis -name …`
//! resolves its switches through the registry (issue #748).  It is deliberately
//! **staged**: the axis methods and the common series/global switches are
//! covered; the full per-series option tables are a follow-up best produced by
//! generating from the ticklecharts `setdef` source rather than by hand.  A
//! method whose options are not yet modelled still resolves as a method call —
//! its `-option value` pairs fall through to the generic option highlighting.

use crate::prelude::*;

/// A value-taking option (`-name value`) — the ticklecharts norm (even
/// booleans are written `-opt True`).
const fn vopt(name: &'static str, detail: &'static str) -> OptionSpec {
    OptionSpec {
        name,
        value: OptionValue::value(""),
        detail,
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    }
}

/// A value-taking option introduced in a specific `ECharts` version.
const fn vopt_since(
    name: &'static str,
    detail: &'static str,
    introduced: &'static str,
) -> OptionSpec {
    OptionSpec {
        name,
        value: OptionValue::value(""),
        detail,
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::introduced_in(introduced),
        min_abbrev: None,
    }
}

/// A closed-set (enumerated) value option (`-type value|category|time|log`).
const fn eopt(name: &'static str, detail: &'static str, values: &'static [ArgValue]) -> OptionSpec {
    OptionSpec {
        name,
        value: OptionValue::enumerated(values, false, ""),
        detail,
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    }
}

/// An instance method with declared options.
const fn method(
    name: &'static str,
    detail: &'static str,
    options: &'static [OptionSpec],
) -> SubCommand {
    SubCommand {
        name,
        detail,
        options,
        ..SubCommand::DEFAULT
    }
}

const AXIS_TYPE_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "value",
        detail: "Numerical axis.",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "category",
        detail: "Category axis.",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "time",
        detail: "Time axis.",
        min_tcl: None,
        code: None,
    },
    ArgValue {
        value: "log",
        detail: "Logarithmic axis.",
        min_tcl: None,
        code: None,
    },
];

/// Options shared by the cartesian / polar axis methods (`Xaxis`, `Yaxis`,
/// `RadiusAxis`, `AngleAxis`).  Extracted from `ticklecharts::xAxis`
/// (`axis.tcl`); the polar axes accept a near-identical set.
const AXIS_OPTIONS: &[OptionSpec] = &[
    vopt("-id", "Component ID."),
    vopt("-show", "Whether to show the axis."),
    eopt("-type", "Axis type.", AXIS_TYPE_VALUES),
    vopt("-data", "Category data (category axis)."),
    vopt("-gridIndex", "Index of the grid this axis belongs to."),
    vopt_since(
        "-alignTicks",
        "Align split lines of multiple axes.",
        "5.3.0",
    ),
    vopt("-position", "Axis position."),
    vopt("-offset", "Offset of this axis from the default position."),
    vopt("-name", "Axis name."),
    vopt("-nameLocation", "Location of the axis name."),
    vopt("-nameTextStyle", "Text style of the axis name."),
    vopt("-nameGap", "Gap between axis name and axis line."),
    vopt("-nameRotate", "Rotation of the axis name."),
    vopt("-nameTruncate", "Truncation rule for the axis name."),
    vopt("-inverse", "Whether the axis is inverted."),
    vopt("-boundaryGap", "Boundary gap on both sides of the axis."),
    vopt("-min", "Minimum value of the axis."),
    vopt("-max", "Maximum value of the axis."),
    vopt("-scale", "Whether the scale excludes zero."),
    vopt("-splitNumber", "Number of segments the axis is split into."),
    vopt("-minInterval", "Minimum gap between split lines."),
    vopt("-maxInterval", "Maximum gap between split lines."),
    vopt("-interval", "Manual gap between split lines."),
    vopt("-logBase", "Base of the logarithm (log axis)."),
    vopt("-silent", "Whether the axis is silent (no events)."),
    vopt("-triggerEvent", "Whether axis labels trigger events."),
    vopt("-axisLine", "Axis line settings."),
    vopt("-axisTick", "Axis tick settings."),
    vopt("-minorTick", "Minor tick settings."),
    vopt("-axisLabel", "Axis label settings."),
    vopt("-splitLine", "Split line settings."),
    vopt("-minorSplitLine", "Minor split line settings."),
    vopt("-splitArea", "Split area settings."),
    vopt("-axisPointer", "Axis pointer settings."),
    vopt("-zlevel", "Z-level for the canvas layer."),
    vopt("-z", "Z-depth within the same z-level."),
    vopt("-polarIndex", "Index of the polar coordinate (polar axes)."),
    vopt("-startAngle", "Starting angle (angle axis)."),
    vopt("-clockwise", "Whether the angle axis increases clockwise."),
];

/// Options common to the series adders (`AddLineSeries`, `AddBarSeries`, …) and
/// the generic `Add` dispatcher.  Extracted from `ticklecharts::lineSeries` /
/// `barSeries` (`series.tcl`); each series type also has type-specific options
/// not yet modelled here.
const SERIES_OPTIONS: &[OptionSpec] = &[
    vopt("-id", "Series ID."),
    vopt("-name", "Series name."),
    vopt("-type", "Series type."),
    vopt_since(
        "-colorBy",
        "Strategy for assigning colours to data.",
        "5.2.0",
    ),
    vopt("-coordinateSystem", "Coordinate system the series uses."),
    vopt("-xAxisIndex", "Index of the x axis to use."),
    vopt("-yAxisIndex", "Index of the y axis to use."),
    vopt("-symbol", "Symbol of the data points."),
    vopt("-symbolSize", "Size of the data-point symbols."),
    vopt("-symbolRotate", "Rotation of the data-point symbols."),
    vopt("-showSymbol", "Whether to show the symbols."),
    vopt("-smooth", "Whether the line is smoothed."),
    vopt("-stack", "Name of the stack group."),
    vopt(
        "-legendHoverLink",
        "Whether to link legend hover to the series.",
    ),
    vopt("-connectNulls", "Whether to connect across null points."),
    vopt(
        "-clip",
        "Whether to clip the series to the coordinate system.",
    ),
    vopt("-step", "Whether the line is a step line."),
    vopt("-lineStyle", "Line style settings."),
    vopt("-areaStyle", "Area style settings."),
    vopt("-itemStyle", "Item (data point) style settings."),
    vopt("-label", "Data label settings."),
    vopt("-labelLine", "Label guide line settings."),
    vopt("-emphasis", "Emphasis (hover) state settings."),
    vopt("-blur", "Blur (faded) state settings."),
    vopt("-select", "Select state settings."),
    vopt("-markPoint", "Mark point settings."),
    vopt("-markLine", "Mark line settings."),
    vopt("-markArea", "Mark area settings."),
    vopt("-tooltip", "Per-series tooltip settings."),
    vopt("-data", "Series data."),
    vopt("-encode", "Data-dimension to axis encoding."),
    vopt("-datasetIndex", "Index of the dataset to use."),
    vopt("-animation", "Whether the series is animated."),
    vopt("-zlevel", "Z-level for the canvas layer."),
    vopt("-z", "Z-depth within the same z-level."),
];

/// Top-level global-option switches accepted by `SetOptions` (`chart.tcl:874`);
/// each delegates to a same-named component proc (`ticklecharts::title`,
/// `legend`, …) with its own option block.
const SETOPTIONS_SWITCHES: &[OptionSpec] = &[
    vopt("-backgroundColor", "Chart background colour."),
    vopt("-color", "Default series colour palette."),
    vopt("-title", "Title component."),
    vopt("-legend", "Legend component."),
    vopt("-grid", "Cartesian grid component."),
    vopt("-tooltip", "Global tooltip component."),
    vopt("-toolbox", "Toolbox component."),
    vopt("-dataZoom", "Data-zoom component."),
    vopt("-visualMap", "Visual-map component."),
    vopt("-dataset", "Dataset component."),
    vopt("-polar", "Polar coordinate component."),
    vopt("-radiusAxis", "Radius axis (polar)."),
    vopt("-angleAxis", "Angle axis (polar)."),
    vopt("-radar", "Radar coordinate component."),
    vopt("-parallel", "Parallel coordinate component."),
    vopt("-parallelAxis", "Parallel axis component."),
    vopt("-singleAxis", "Single axis component."),
    vopt("-brush", "Brush (selection) component."),
    vopt("-axisPointer", "Global axis pointer component."),
    vopt("-geo", "Geographic coordinate component."),
    vopt("-calendar", "Calendar coordinate component."),
    vopt("-aria", "Accessibility (ARIA) component."),
    vopt("-graphic", "Graphic-element component."),
    vopt("-textStyle", "Global text style."),
    vopt("-animation", "Whether global animation is enabled."),
    vopt("-darkMode", "Whether dark mode is enabled."),
];

/// The `chart` class methods (`chart.tcl`).  The axis and `SetOptions`
/// switches are modelled; the series adders share [`SERIES_OPTIONS`]; the
/// render / export accessors take no `-option` switches.
const CHART_METHODS: &[SubCommand] = &[
    method("Xaxis", "Configure the cartesian X axis.", AXIS_OPTIONS),
    method("Yaxis", "Configure the cartesian Y axis.", AXIS_OPTIONS),
    method(
        "RadiusAxis",
        "Configure the polar radius axis.",
        AXIS_OPTIONS,
    ),
    method("AngleAxis", "Configure the polar angle axis.", AXIS_OPTIONS),
    method("SingleAxis", "Configure a single axis.", AXIS_OPTIONS),
    method("ParallelAxis", "Configure a parallel axis.", AXIS_OPTIONS),
    method(
        "SetOptions",
        "Set global chart options.",
        SETOPTIONS_SWITCHES,
    ),
    method("AddLineSeries", "Add a line series.", SERIES_OPTIONS),
    method("AddBarSeries", "Add a bar series.", SERIES_OPTIONS),
    method("AddPieSeries", "Add a pie series.", SERIES_OPTIONS),
    method("AddScatterSeries", "Add a scatter series.", SERIES_OPTIONS),
    method("AddFunnelSeries", "Add a funnel series.", SERIES_OPTIONS),
    method("AddRadarSeries", "Add a radar series.", SERIES_OPTIONS),
    method("AddHeatmapSeries", "Add a heatmap series.", SERIES_OPTIONS),
    method(
        "AddSunburstSeries",
        "Add a sunburst series.",
        SERIES_OPTIONS,
    ),
    method("AddTreeSeries", "Add a tree series.", SERIES_OPTIONS),
    method(
        "AddThemeRiverSeries",
        "Add a theme-river series.",
        SERIES_OPTIONS,
    ),
    method("AddSankeySeries", "Add a sankey series.", SERIES_OPTIONS),
    method(
        "AddPictorialBarSeries",
        "Add a pictorial-bar series.",
        SERIES_OPTIONS,
    ),
    method(
        "AddCandlestickSeries",
        "Add a candlestick series.",
        SERIES_OPTIONS,
    ),
    method(
        "AddParallelSeries",
        "Add a parallel series.",
        SERIES_OPTIONS,
    ),
    method("AddGaugeSeries", "Add a gauge series.", SERIES_OPTIONS),
    method("AddGraphSeries", "Add a graph series.", SERIES_OPTIONS),
    method(
        "AddWordCloudSeries",
        "Add a word-cloud series.",
        SERIES_OPTIONS,
    ),
    method("AddBoxPlotSeries", "Add a box-plot series.", SERIES_OPTIONS),
    method("AddTreeMapSeries", "Add a treemap series.", SERIES_OPTIONS),
    method("AddMapSeries", "Add a map series.", SERIES_OPTIONS),
    method("AddLinesSeries", "Add a lines series.", SERIES_OPTIONS),
    method("Add", "Add a series by type name.", SERIES_OPTIONS),
    method("AddGraphic", "Add a graphic element.", &[]),
    method("RadarCoordinate", "Configure the radar coordinate.", &[]),
    method("AddJSON", "Add raw JSON options.", &[]),
    method("Render", "Render the chart to an HTML file.", &[]),
    method("toHTML", "Return the chart as an HTML string.", &[]),
    method("toJSON", "Return the chart options as JSON.", &[]),
    method("get", "Get the chart option dictionary.", &[]),
    method("options", "Get the chart options.", &[]),
];

static CHART_CLASS: ObjectClassSpec = ObjectClassSpec {
    class_name: "ticklecharts::chart",
    instance_methods: CHART_METHODS,
    superclasses: &[],
    allow_unknown_methods: false,
};

/// The `chart3D` class methods (`chart3D.tcl`).  Options for the 3D axes and
/// series are not yet modelled — the methods resolve as calls and their
/// switches fall through to generic option highlighting.
const CHART3D_METHODS: &[SubCommand] = &[
    method("Xaxis3D", "Configure the 3D X axis.", &[]),
    method("Yaxis3D", "Configure the 3D Y axis.", &[]),
    method("Zaxis3D", "Configure the 3D Z axis.", &[]),
    method("AddLine3DSeries", "Add a 3D line series.", &[]),
    method("AddBar3DSeries", "Add a 3D bar series.", &[]),
    method("AddSurfaceSeries", "Add a surface series.", &[]),
    method("AddScatter3DSeries", "Add a 3D scatter series.", &[]),
    method("AddLines3DSeries", "Add a 3D lines series.", &[]),
    method("Add", "Add a 3D series by type name.", &[]),
    method(
        "SetOptions",
        "Set global chart options.",
        SETOPTIONS_SWITCHES,
    ),
    method("Render", "Render the chart to an HTML file.", &[]),
    method("toHTML", "Return the chart as an HTML string.", &[]),
    method("toJSON", "Return the chart options as JSON.", &[]),
];

static CHART3D_CLASS: ObjectClassSpec = ObjectClassSpec {
    class_name: "ticklecharts::chart3D",
    instance_methods: CHART3D_METHODS,
    superclasses: &[],
    allow_unknown_methods: false,
};

/// The `timeline` class methods (`timeline.tcl`); render / export are
/// re-exported from `chart`.
const TIMELINE_METHODS: &[SubCommand] = &[
    method("Add", "Add a chart to the timeline.", &[]),
    method("SetOptions", "Set timeline options.", SETOPTIONS_SWITCHES),
    method("Render", "Render the timeline to an HTML file.", &[]),
    method("toHTML", "Return the timeline as an HTML string.", &[]),
    method("toJSON", "Return the timeline options as JSON.", &[]),
];

static TIMELINE_CLASS: ObjectClassSpec = ObjectClassSpec {
    class_name: "ticklecharts::timeline",
    instance_methods: TIMELINE_METHODS,
    superclasses: &[],
    allow_unknown_methods: false,
};

/// The `Gridlayout` class methods (`layout.tcl`); most accessors are copied
/// from `chart`.
const GRIDLAYOUT_METHODS: &[SubCommand] = &[
    method("Add", "Add a chart to the grid layout.", &[]),
    method(
        "SetGlobalOptions",
        "Set global layout options.",
        SETOPTIONS_SWITCHES,
    ),
    method("Render", "Render the layout to an HTML file.", &[]),
    method("toHTML", "Return the layout as an HTML string.", &[]),
    method("toJSON", "Return the layout options as JSON.", &[]),
];

static GRIDLAYOUT_CLASS: ObjectClassSpec = ObjectClassSpec {
    class_name: "ticklecharts::Gridlayout",
    instance_methods: GRIDLAYOUT_METHODS,
    superclasses: &[],
    allow_unknown_methods: false,
};

/// The `new` / `create` constructor subcommands shared by every class factory.
const CTOR_SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "new",
        detail: "Create a new object with an auto-generated command name.",
        synopsis: "new",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "create",
        detail: "Create a new object with the given command name.",
        synopsis: "create name",
        ..SubCommand::DEFAULT
    },
];

fn class_spec(
    name: &'static str,
    summary: &'static str,
    object_class: &'static ObjectClassSpec,
) -> CommandSpec {
    CommandSpec {
        name,
        // A desktop Tcl charting package — available in every Tcl version but
        // never in the iRules / iApps / EDA dialects (where it has no place and
        // would otherwise inflate their valid-command counts).
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        subcommands: CTOR_SUBCOMMANDS,
        allow_unknown_subcommands: true,
        required_package: Some("ticklecharts"),
        object_class: Some(object_class),
        hover: Some(HoverSnippet {
            summary,
            synopsis: &["ticklecharts::CLASS new", "ticklecharts::CLASS create name"],
            snippet: "A ticklecharts (Apache ECharts) class. Instantiate with `new`, then configure the returned object handle with its methods.",
            source: "ticklecharts (nico-robert)",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}

/// All command specs contributed by the `ticklecharts` package.
#[must_use]
pub fn ticklecharts_command_specs() -> Vec<CommandSpec> {
    vec![
        class_spec(
            "ticklecharts::chart",
            "Main 2D ticklecharts chart.",
            &CHART_CLASS,
        ),
        class_spec(
            "ticklecharts::chart3D",
            "3D ticklecharts chart.",
            &CHART3D_CLASS,
        ),
        class_spec(
            "ticklecharts::timeline",
            "Animated multi-chart timeline.",
            &TIMELINE_CLASS,
        ),
        class_spec(
            "ticklecharts::Gridlayout",
            "Multi-chart grid layout.",
            &GRIDLAYOUT_CLASS,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use crate::CommandRegistry;

    #[test]
    fn chart_factory_and_methods_resolve() {
        let reg = CommandRegistry::build_default();
        // The factory command carries the object-class metadata.
        let class = reg
            .object_class("ticklecharts::chart")
            .expect("chart factory has an object class");
        assert_eq!(class.class_name, "ticklecharts::chart");
        // A declared instance method resolves, with its options.
        let xaxis = reg
            .instance_method("ticklecharts::chart", "Xaxis")
            .expect("Xaxis method resolves");
        assert!(
            xaxis.options.iter().any(|o| o.name == "-name"),
            "Xaxis declares -name"
        );
        assert!(
            xaxis.options.iter().any(|o| o.name == "-splitLine"),
            "Xaxis declares -splitLine"
        );
        // An undeclared method does not resolve.
        assert!(
            reg.instance_method("ticklecharts::chart", "NoSuchMethod")
                .is_none(),
            "unknown method must not resolve"
        );
    }
}
