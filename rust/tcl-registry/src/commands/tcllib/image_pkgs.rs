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

//! tcllib document / image / protocol packages — `Markdown`, `oauth`,
//! `imap4`, `mapproj`, `gpx`, `png`, `jpeg`, and `tiff`.
//!
//! Public command surface from the upstream tcllib 2.0 manual pages.  Command
//! names, arity bounds, synopses, and summaries are derived from those pages.
//! Requires Tcl 8.5+.

use crate::prelude::*;

/// A row in a package command table: name, arity, synopsis, and summary.
type Row = (&'static str, Arity, &'static [&'static str], &'static str);

/// Build (non-pure) command specs for a package from its table.
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

/// The `Markdown` package.
const MARKDOWN_CMDS: &[Row] = &[
    (
        "Markdown::convert",
        Arity::exact(1),
        &["Markdown::convert markdown"],
        "This command takes in a block of Markdown text, and returns a block of HTML.",
    ),
    (
        "Markdown::register",
        Arity::exact(2),
        &["Markdown::register langspec converter"],
        "Register a language specific converter for prettifying a code block (e.g.",
    ),
    (
        "Markdown::get_lang_counter",
        Arity::exact(0),
        &["Markdown::get_lang_counter"],
        "Return a dict of language specifier and number of occurrences in fenced code blocks.",
    ),
    (
        "Markdown::reset_lang_counter",
        Arity::exact(0),
        &["Markdown::reset_lang_counter"],
        "Reset the language counters.",
    ),
];

/// The `oauth` package.
const OAUTH_CMDS: &[Row] = &[
    (
        "oauth::config",
        Arity::at_least(0),
        &["oauth::config"],
        "When this command is invoked without arguments it returns a dictionary containing the current values of all.",
    ),
    (
        "oauth::header",
        Arity::at_least(1),
        &["oauth::header baseURL"],
        "This command is the base signature creator.",
    ),
    (
        "oauth::query",
        Arity::at_least(1),
        &["oauth::query baseURL"],
        "This procedure will use the settings made with ::oauth::config to create the basic authentication and then send.",
    ),
];

/// The `imap4` package.
const IMAP4_CMDS: &[Row] = &[
    (
        "imap4::open",
        Arity::at_least(1),
        &["imap4::open hostname"],
        "Open a new IMAP connection and initalize the handler, the imap communication channel (handler) is returned.",
    ),
    (
        "imap4::starttls",
        Arity::exact(1),
        &["imap4::starttls chan"],
        "Use this when tasked with connecting to an unsecure port which must be changed to a secure port prior to user.",
    ),
    (
        "imap4::login",
        Arity::exact(3),
        &["imap4::login chan user pass"],
        "Login using the IMAP LOGIN command, 0 is returned on successful login.",
    ),
    (
        "imap4::folders",
        Arity::at_least(1),
        &["imap4::folders chan"],
        "Get list of matching folders, 0 is returned on success.",
    ),
    (
        "imap4::select",
        Arity::at_least(1),
        &["imap4::select chan"],
        "Select a mailbox, 0 is returned on success.",
    ),
    (
        "imap4::examine",
        Arity::at_least(1),
        &["imap4::examine chan"],
        "Examines a mailbox, read-only equivalent of ::imap4::select.",
    ),
    (
        "imap4::fetch",
        Arity::at_least(2),
        &["imap4::fetch chan range"],
        "Fetch attributes from messages.",
    ),
    (
        "imap4::noop",
        Arity::exact(1),
        &["imap4::noop chan"],
        "Send NOOP command to server.",
    ),
    (
        "imap4::check",
        Arity::exact(1),
        &["imap4::check chan"],
        "Send CHECK command to server.",
    ),
    (
        "imap4::folderinfo",
        Arity::at_least(1),
        &["imap4::folderinfo chan"],
        "Get information on the recently selected folderlist.",
    ),
    (
        "imap4::msginfo",
        Arity::at_least(2),
        &["imap4::msginfo chan msgid"],
        "Get information (from previously collected using fetch) from a given msgid.",
    ),
    (
        "imap4::mboxinfo",
        Arity::at_least(1),
        &["imap4::mboxinfo chan"],
        "Get information on the currently selected mailbox.",
    ),
    (
        "imap4::isableto",
        Arity::at_least(1),
        &["imap4::isableto chan"],
        "Test for capability.",
    ),
    (
        "imap4::create",
        Arity::exact(2),
        &["imap4::create chan mailbox"],
        "Create a new mailbox.",
    ),
    (
        "imap4::delete",
        Arity::exact(2),
        &["imap4::delete chan mailbox"],
        "Delete a new mailbox.",
    ),
    (
        "imap4::rename",
        Arity::exact(3),
        &["imap4::rename chan oldname newname"],
        "Rename a new mailbox.",
    ),
    (
        "imap4::subscribe",
        Arity::exact(2),
        &["imap4::subscribe chan mailbox"],
        "Subscribe a new mailbox.",
    ),
    (
        "imap4::unsubscribe",
        Arity::exact(2),
        &["imap4::unsubscribe chan mailbox"],
        "Unsubscribe a new mailbox.",
    ),
    (
        "imap4::search",
        Arity::at_least(2),
        &["imap4::search chan expr"],
        "Search for mails matching search criterions, 0 is returned on success.",
    ),
    (
        "imap4::close",
        Arity::exact(1),
        &["imap4::close chan"],
        "Close the mailbox.",
    ),
    (
        "imap4::cleanup",
        Arity::exact(1),
        &["imap4::cleanup chan"],
        "Destroy an IMAP connection and free the used space.",
    ),
    (
        "imap4::debugmode",
        Arity::at_least(1),
        &["imap4::debugmode chan"],
        "Switch client into command line debug mode.",
    ),
    (
        "imap4::store",
        Arity::exact(4),
        &["imap4::store chan range data flaglist"],
        "Alters data associated with a message in the selected mailbox.",
    ),
    (
        "imap4::expunge",
        Arity::exact(1),
        &["imap4::expunge chan"],
        "Permanently removes all messages that have the Deleted flag set from the currently selected mailbox, without the.",
    ),
    (
        "imap4::copy",
        Arity::exact(3),
        &["imap4::copy chan msgid mailbox"],
        "Copies the specified message (identified by its message number) to the named mailbox, i.e.",
    ),
    (
        "imap4::logout",
        Arity::exact(1),
        &["imap4::logout chan"],
        "Informs the server that the client is done with the connection and closes the network connection.",
    ),
];

/// The `mapproj` package.
const MAPPROJ_CMDS: &[Row] = &[
    (
        "mapproj::toPlateCarree",
        Arity::exact(4),
        &["mapproj::toPlateCarree lambda_0 phi_0 lambda phi"],
        "Converts to the plate carre (cylindrical equidistant) projection.",
    ),
    (
        "mapproj::fromPlateCarree",
        Arity::exact(4),
        &["mapproj::fromPlateCarree lambda_0 phi_0 x y"],
        "Converts from the plate carre (cylindrical equidistant) projection.",
    ),
    (
        "mapproj::toCylindricalEqualArea",
        Arity::exact(4),
        &["mapproj::toCylindricalEqualArea lambda_0 phi_0 lambda phi"],
        "Converts to the cylindrical equal-area projection.",
    ),
    (
        "mapproj::fromCylindricalEqualArea",
        Arity::exact(4),
        &["mapproj::fromCylindricalEqualArea lambda_0 phi_0 x y"],
        "Converts from the cylindrical equal-area projection.",
    ),
    (
        "mapproj::toMercator",
        Arity::exact(4),
        &["mapproj::toMercator lambda_0 phi_0 lambda phi"],
        "Converts to the Mercator (cylindrical conformal) projection.",
    ),
    (
        "mapproj::fromMercator",
        Arity::exact(4),
        &["mapproj::fromMercator lambda_0 phi_0 x y"],
        "Converts from the Mercator (cylindrical conformal) projection.",
    ),
    (
        "mapproj::toMillerCylindrical",
        Arity::exact(3),
        &["mapproj::toMillerCylindrical lambda_0 lambda phi"],
        "Converts to the Miller Cylindrical projection.",
    ),
    (
        "mapproj::fromMillerCylindrical",
        Arity::exact(3),
        &["mapproj::fromMillerCylindrical lambda_0 x y"],
        "Converts from the Miller Cylindrical projection.",
    ),
    (
        "mapproj::toSinusoidal",
        Arity::exact(4),
        &["mapproj::toSinusoidal lambda_0 phi_0 lambda phi"],
        "Converts to the sinusoidal (Sanson-Flamsteed) projection.",
    ),
    (
        "mapproj::fromSinusoidal",
        Arity::exact(4),
        &["mapproj::fromSinusoidal lambda_0 phi_0 x y"],
        "Converts from the sinusoidal (Sanson-Flamsteed) projection.",
    ),
    (
        "mapproj::toMollweide",
        Arity::exact(3),
        &["mapproj::toMollweide lambda_0 lambda phi"],
        "Converts to the Mollweide projection.",
    ),
    (
        "mapproj::fromMollweide",
        Arity::exact(3),
        &["mapproj::fromMollweide lambda_0 x y"],
        "Converts from the Mollweide projection.",
    ),
    (
        "mapproj::toEckertIV",
        Arity::exact(3),
        &["mapproj::toEckertIV lambda_0 lambda phi"],
        "Converts to the Eckert IV projection.",
    ),
    (
        "mapproj::fromEckertIV",
        Arity::exact(3),
        &["mapproj::fromEckertIV lambda_0 x y"],
        "Converts from the Eckert IV projection.",
    ),
    (
        "mapproj::toEckertVI",
        Arity::exact(3),
        &["mapproj::toEckertVI lambda_0 lambda phi"],
        "Converts to the Eckert VI projection.",
    ),
    (
        "mapproj::fromEckertVI",
        Arity::exact(3),
        &["mapproj::fromEckertVI lambda_0 x y"],
        "Converts from the Eckert VI projection.",
    ),
    (
        "mapproj::toRobinson",
        Arity::exact(3),
        &["mapproj::toRobinson lambda_0 lambda phi"],
        "Converts to the Robinson projection.",
    ),
    (
        "mapproj::fromRobinson",
        Arity::exact(3),
        &["mapproj::fromRobinson lambda_0 x y"],
        "Converts from the Robinson projection.",
    ),
    (
        "mapproj::toCassini",
        Arity::exact(4),
        &["mapproj::toCassini lambda_0 phi_0 lambda phi"],
        "Converts to the Cassini (transverse cylindrical equidistant) projection.",
    ),
    (
        "mapproj::fromCassini",
        Arity::exact(4),
        &["mapproj::fromCassini lambda_0 phi_0 x y"],
        "Converts from the Cassini (transverse cylindrical equidistant) projection.",
    ),
    (
        "mapproj::toPeirceQuincuncial",
        Arity::exact(3),
        &["mapproj::toPeirceQuincuncial lambda_0 lambda phi"],
        "Converts to the Peirce Quincuncial Projection.",
    ),
    (
        "mapproj::fromPeirceQuincuncial",
        Arity::exact(3),
        &["mapproj::fromPeirceQuincuncial lambda_0 x y"],
        "Converts from the Peirce Quincuncial Projection.",
    ),
    (
        "mapproj::toOrthographic",
        Arity::exact(4),
        &["mapproj::toOrthographic lambda_0 phi_0 lambda phi"],
        "Converts to the orthographic projection.",
    ),
    (
        "mapproj::fromOrthographic",
        Arity::exact(4),
        &["mapproj::fromOrthographic lambda_0 phi_0 x y"],
        "Converts from the orthographic projection.",
    ),
    (
        "mapproj::toStereographic",
        Arity::exact(4),
        &["mapproj::toStereographic lambda_0 phi_0 lambda phi"],
        "Converts to the stereographic (azimuthal conformal) projection.",
    ),
    (
        "mapproj::fromStereographic",
        Arity::exact(4),
        &["mapproj::fromStereographic lambda_0 phi_0 x y"],
        "Converts from the stereographic (azimuthal conformal) projection.",
    ),
    (
        "mapproj::toGnomonic",
        Arity::exact(4),
        &["mapproj::toGnomonic lambda_0 phi_0 lambda phi"],
        "Converts to the gnomonic projection.",
    ),
    (
        "mapproj::fromGnomonic",
        Arity::exact(4),
        &["mapproj::fromGnomonic lambda_0 phi_0 x y"],
        "Converts from the gnomonic projection.",
    ),
    (
        "mapproj::toAzimuthalEquidistant",
        Arity::exact(4),
        &["mapproj::toAzimuthalEquidistant lambda_0 phi_0 lambda phi"],
        "Converts to the azimuthal equidistant projection.",
    ),
    (
        "mapproj::fromAzimuthalEquidistant",
        Arity::exact(4),
        &["mapproj::fromAzimuthalEquidistant lambda_0 phi_0 x y"],
        "Converts from the azimuthal equidistant projection.",
    ),
    (
        "mapproj::toLambertAzimuthalEqualArea",
        Arity::exact(4),
        &["mapproj::toLambertAzimuthalEqualArea lambda_0 phi_0 lambda phi"],
        "Converts to the Lambert azimuthal equal-area projection.",
    ),
    (
        "mapproj::fromLambertAzimuthalEqualArea",
        Arity::exact(4),
        &["mapproj::fromLambertAzimuthalEqualArea lambda_0 phi_0 x y"],
        "Converts from the Lambert azimuthal equal-area projection.",
    ),
    (
        "mapproj::toHammer",
        Arity::exact(3),
        &["mapproj::toHammer lambda_0 lambda phi"],
        "Converts to the Hammer projection.",
    ),
    (
        "mapproj::fromHammer",
        Arity::exact(3),
        &["mapproj::fromHammer lambda_0 x y"],
        "Converts from the Hammer projection.",
    ),
    (
        "mapproj::toConicEquidistant",
        Arity::exact(6),
        &["mapproj::toConicEquidistant lambda_0 phi_0 phi_1 phi_2 lambda phi"],
        "Converts to the conic equidistant projection.",
    ),
    (
        "mapproj::fromConicEquidistant",
        Arity::exact(6),
        &["mapproj::fromConicEquidistant lambda_0 phi_0 phi_1 phi_2 x y"],
        "Converts from the conic equidistant projection.",
    ),
    (
        "mapproj::toAlbersEqualAreaConic",
        Arity::exact(6),
        &["mapproj::toAlbersEqualAreaConic lambda_0 phi_0 phi_1 phi_2 lambda phi"],
        "Converts to the Albers equal-area conic projection.",
    ),
    (
        "mapproj::fromAlbersEqualAreaConic",
        Arity::exact(6),
        &["mapproj::fromAlbersEqualAreaConic lambda_0 phi_0 phi_1 phi_2 x y"],
        "Converts from the Albers equal-area conic projection.",
    ),
    (
        "mapproj::toLambertConformalConic",
        Arity::exact(6),
        &["mapproj::toLambertConformalConic lambda_0 phi_0 phi_1 phi_2 lambda phi"],
        "Converts to the Lambert conformal conic projection.",
    ),
    (
        "mapproj::fromLambertConformalConic",
        Arity::exact(6),
        &["mapproj::fromLambertConformalConic lambda_0 phi_0 phi_1 phi_2 x y"],
        "Converts from the Lambert conformal conic projection.",
    ),
    (
        "mapproj::toLambertCylindricalEqualArea",
        Arity::exact(4),
        &["mapproj::toLambertCylindricalEqualArea lambda_0 phi_0 lambda phi"],
        "Converts to the Lambert cylindrical equal area projection.",
    ),
    (
        "mapproj::fromLambertCylindricalEqualArea",
        Arity::exact(4),
        &["mapproj::fromLambertCylindricalEqualArea lambda_0 phi_0 x y"],
        "Converts from the Lambert cylindrical equal area projection.",
    ),
    (
        "mapproj::toBehrmann",
        Arity::exact(4),
        &["mapproj::toBehrmann lambda_0 phi_0 lambda phi"],
        "Converts to the Behrmann cylindrical equal area projection.",
    ),
    (
        "mapproj::fromBehrmann",
        Arity::exact(4),
        &["mapproj::fromBehrmann lambda_0 phi_0 x y"],
        "Converts from the Behrmann cylindrical equal area projection.",
    ),
    (
        "mapproj::toTrystanEdwards",
        Arity::exact(4),
        &["mapproj::toTrystanEdwards lambda_0 phi_0 lambda phi"],
        "Converts to the Trystan Edwards cylindrical equal area projection.",
    ),
    (
        "mapproj::fromTrystanEdwards",
        Arity::exact(4),
        &["mapproj::fromTrystanEdwards lambda_0 phi_0 x y"],
        "Converts from the Trystan Edwards cylindrical equal area projection.",
    ),
    (
        "mapproj::toHoboDyer",
        Arity::exact(4),
        &["mapproj::toHoboDyer lambda_0 phi_0 lambda phi"],
        "Converts to the Hobo-Dyer cylindrical equal area projection.",
    ),
    (
        "mapproj::fromHoboDyer",
        Arity::exact(4),
        &["mapproj::fromHoboDyer lambda_0 phi_0 x y"],
        "Converts from the Hobo-Dyer cylindrical equal area projection.",
    ),
    (
        "mapproj::toGallPeters",
        Arity::exact(4),
        &["mapproj::toGallPeters lambda_0 phi_0 lambda phi"],
        "Converts to the Gall-Peters cylindrical equal area projection.",
    ),
    (
        "mapproj::fromGallPeters",
        Arity::exact(4),
        &["mapproj::fromGallPeters lambda_0 phi_0 x y"],
        "Converts from the Gall-Peters cylindrical equal area projection.",
    ),
    (
        "mapproj::toBalthasart",
        Arity::exact(4),
        &["mapproj::toBalthasart lambda_0 phi_0 lambda phi"],
        "Converts to the Balthasart cylindrical equal area projection.",
    ),
    (
        "mapproj::fromBalthasart",
        Arity::exact(4),
        &["mapproj::fromBalthasart lambda_0 phi_0 x y"],
        "Converts from the Balthasart cylindrical equal area projection.",
    ),
];

/// The `gpx` package.
const GPX_CMDS: &[Row] = &[
    (
        "gpx::Create",
        Arity::at_least(1),
        &["gpx::Create gpxFilename"],
        "The ::gpx::Create is the first command called to process GPX data.",
    ),
    (
        "gpx::Cleanup",
        Arity::exact(1),
        &["gpx::Cleanup token"],
        "This procedure cleans up resources associated with token.",
    ),
    (
        "gpx::GetGPXMetadata",
        Arity::exact(1),
        &["gpx::GetGPXMetadata token"],
        "This procedure returns a dictionary of the metadata associated with the GPX data identified by token.",
    ),
    (
        "gpx::GetWaypointCount",
        Arity::exact(1),
        &["gpx::GetWaypointCount token"],
        "This procedure returns the number of waypoints defined in the GPX data identified by token.",
    ),
    (
        "gpx::GetAllWaypoints",
        Arity::exact(1),
        &["gpx::GetAllWaypoints token"],
        "This procedure returns the a list of waypoints defined in the GPX data identified by token.",
    ),
    (
        "gpx::GetTrackCount",
        Arity::exact(1),
        &["gpx::GetTrackCount token"],
        "This procedure returns the number of tracks defined in the GPX data identified by token.",
    ),
    (
        "gpx::GetTrackMetadata",
        Arity::exact(2),
        &["gpx::GetTrackMetadata token whichTrack"],
        "This procedure returns a dictionary of the metadata associated track number whichTrack (1 based) in the GPX data.",
    ),
    (
        "gpx::GetTrackPoints",
        Arity::exact(2),
        &["gpx::GetTrackPoints token whichTrack"],
        "The procedure returns a list of track points comprising track number whichTrack (1 based) in the GPX data.",
    ),
    (
        "gpx::GetRouteCount",
        Arity::exact(1),
        &["gpx::GetRouteCount token"],
        "This procedure returns the number of routes defined in the GPX data identified by token.",
    ),
    (
        "gpx::GetRouteMetadata",
        Arity::exact(2),
        &["gpx::GetRouteMetadata token whichRoute"],
        "This procedure returns a dictionary of the metadata associated route number whichRoute (1 based) in the GPX data.",
    ),
    (
        "gpx::GetRoutePoints",
        Arity::exact(2),
        &["gpx::GetRoutePoints token whichRoute"],
        "The procedure returns a list of route points comprising route number whichRoute (1 based) in the GPX data.",
    ),
];

/// The `png` package.
const PNG_CMDS: &[Row] = &[
    (
        "png::validate",
        Arity::exact(1),
        &["png::validate file"],
        "Returns a value indicating if file is a valid PNG file.",
    ),
    (
        "png::isPNG",
        Arity::exact(1),
        &["png::isPNG file"],
        "Returns a boolean value indicating if the file file starts with a PNG signature.",
    ),
    (
        "png::imageInfo",
        Arity::exact(1),
        &["png::imageInfo file"],
        "Returns a dictionary with keys width, height, depth, color, compression, filter, and interlace.",
    ),
    (
        "png::getTimestamp",
        Arity::exact(1),
        &["png::getTimestamp file"],
        "Returns the epoch time if a timestamp chunk is found in the PNG image contained in the file, otherwise returns.",
    ),
    (
        "png::setTimestamp",
        Arity::exact(2),
        &["png::setTimestamp file time"],
        "Writes a new timestamp to the file, either replacing the old timestamp, or adding one just before the data chunks.",
    ),
    (
        "png::getComments",
        Arity::exact(1),
        &["png::getComments file"],
        "Currently supports only uncompressed comments.",
    ),
    (
        "png::removeComments",
        Arity::exact(1),
        &["png::removeComments file"],
        "Removes all comments from the PNG image in file.",
    ),
    (
        "png::addComment",
        Arity::at_least(3),
        &["png::addComment file keyword text"],
        "Adds a plain text comment to the PNG image in file, just before the first data chunk.",
    ),
    (
        "png::getPixelDimension",
        Arity::exact(1),
        &["png::getPixelDimension file"],
        "Returns a dictionary with keys ppux, ppuy and unit if the information is present.",
    ),
    (
        "png::image",
        Arity::exact(1),
        &["png::image file"],
        "Given a PNG file returns the image in the list of scanlines format used by Tk_GetColor.",
    ),
    (
        "png::write",
        Arity::exact(2),
        &["png::write file data"],
        "Takes a list of scanlines in the Tk_GetColor format and writes the represented image to file.",
    ),
];

/// The `jpeg` package.
const JPEG_CMDS: &[Row] = &[
    (
        "jpeg::isJPEG",
        Arity::exact(1),
        &["jpeg::isJPEG file"],
        "Returns a boolean value indicating if file is a JPEG image.",
    ),
    (
        "jpeg::imageInfo",
        Arity::exact(1),
        &["jpeg::imageInfo file"],
        "Returns a dictionary with keys version, units, xdensity, ydensity, xthumb, and ythumb.",
    ),
    (
        "jpeg::dimensions",
        Arity::exact(1),
        &["jpeg::dimensions file"],
        "Returns the dimensions of the JPEG file as a list of the horizontal and vertical pixel count.",
    ),
    (
        "jpeg::getThumbnail",
        Arity::exact(1),
        &["jpeg::getThumbnail file"],
        "This procedure will return the binary thumbnail image data, if a JPEG thumbnail is included in file, and the.",
    ),
    (
        "jpeg::getExif",
        Arity::at_least(1),
        &["jpeg::getExif file"],
        "Section must be one of main or thumbnail.",
    ),
    (
        "jpeg::getExifFromChannel",
        Arity::at_least(1),
        &["jpeg::getExifFromChannel channel"],
        "This command is as per ::jpeg::getExif except that it uses a previously opened channel.",
    ),
    (
        "jpeg::formatExif",
        Arity::exact(1),
        &["jpeg::formatExif keys"],
        "Takes a list of key-value pairs as returned by getExif and formats many of the values into a more human readable.",
    ),
    (
        "jpeg::exifKeys",
        Arity::exact(0),
        &["jpeg::exifKeys"],
        "Returns a list of the EXIF keys which are currently understood.",
    ),
    (
        "jpeg::removeExif",
        Arity::exact(1),
        &["jpeg::removeExif file"],
        "Removes the Exif data segment from the specified file and replaces it with a standard JFIF segment.",
    ),
    (
        "jpeg::stripJPEG",
        Arity::exact(1),
        &["jpeg::stripJPEG file"],
        "Removes all metadata from the JPEG file leaving only the image.",
    ),
    (
        "jpeg::getComments",
        Arity::exact(1),
        &["jpeg::getComments file"],
        "Returns a list containing all the JPEG comments found in the file.",
    ),
    (
        "jpeg::addComment",
        Arity::at_least(2),
        &["jpeg::addComment file text..."],
        "Adds one or more plain text comments to the JPEG image in file.",
    ),
    (
        "jpeg::removeComments",
        Arity::exact(1),
        &["jpeg::removeComments file"],
        "Removes all comments from the file specified.",
    ),
    (
        "jpeg::replaceComment",
        Arity::exact(2),
        &["jpeg::replaceComment file text"],
        "Replaces the first comment in the file with the new text.",
    ),
    (
        "jpeg::debug",
        Arity::exact(1),
        &["jpeg::debug file"],
        "Prints everything we know about the given file in a nice format.",
    ),
    (
        "jpeg::markers",
        Arity::exact(1),
        &["jpeg::markers channel"],
        "This is an internal helper command, we document it for use by advanced users of the package.",
    ),
];

/// The `tiff` package.
const TIFF_CMDS: &[Row] = &[
    (
        "tiff::isTIFF",
        Arity::exact(1),
        &["tiff::isTIFF file"],
        "Returns a boolean value indicating if file is a TIFF image.",
    ),
    (
        "tiff::byteOrder",
        Arity::exact(1),
        &["tiff::byteOrder file"],
        "Returns either big or little.",
    ),
    (
        "tiff::numImages",
        Arity::exact(1),
        &["tiff::numImages file"],
        "Returns the number of images in file.",
    ),
    (
        "tiff::dimensions",
        Arity::at_least(1),
        &["tiff::dimensions file image"],
        "Returns the dimensions of image number image in file as a list of the horizontal and vertical pixel count.",
    ),
    (
        "tiff::imageInfo",
        Arity::at_least(1),
        &["tiff::imageInfo file image"],
        "Returns a dictionary with keys ImageWidth, ImageLength, BitsPerSample, Compression, PhotometricInterpretation.",
    ),
    (
        "tiff::entries",
        Arity::at_least(1),
        &["tiff::entries file image"],
        "Returns a list of all entries in the given file and image in hexadecimal format.",
    ),
    (
        "tiff::getEntry",
        Arity::at_least(2),
        &["tiff::getEntry file entry image"],
        "Returns the value of entry from image image in the TIFF file.",
    ),
    (
        "tiff::addEntry",
        Arity::at_least(2),
        &["tiff::addEntry file entry image"],
        "Adds the specified entries to the image named by image (default 0), or optionally all.",
    ),
    (
        "tiff::deleteEntry",
        Arity::at_least(2),
        &["tiff::deleteEntry file entry image"],
        "Deletes the specified entries from the image named by image (default 0), or optionally all.",
    ),
    (
        "tiff::getImage",
        Arity::at_least(1),
        &["tiff::getImage file image"],
        "Returns the name of a Tk image containing the image at index image from file Throws an error if file is not a.",
    ),
    (
        "tiff::writeImage",
        Arity::at_least(2),
        &["tiff::writeImage image file entry"],
        "Writes the contents of the Tk image image to a tiff file file.",
    ),
    (
        "tiff::nametotag",
        Arity::exact(1),
        &["tiff::nametotag names"],
        "Returns a list with names translated from string to 4 digit format.",
    ),
    (
        "tiff::tagtoname",
        Arity::exact(1),
        &["tiff::tagtoname tags"],
        "Returns a list with tags translated from 4 digit to string format.",
    ),
    (
        "tiff::debug",
        Arity::exact(1),
        &["tiff::debug file"],
        "Prints everything we know about the given file in a nice format.",
    ),
];

/// All document / image / protocol package command specs.
pub fn specs() -> Vec<CommandSpec> {
    let mut specs = Vec::new();
    specs.extend(rows("Markdown", MARKDOWN_CMDS));
    specs.extend(rows("oauth", OAUTH_CMDS));
    specs.extend(rows("imap4", IMAP4_CMDS));
    specs.extend(rows("mapproj", MAPPROJ_CMDS));
    specs.extend(rows("gpx", GPX_CMDS));
    specs.extend(rows("png", PNG_CMDS));
    specs.extend(rows("jpeg", JPEG_CMDS));
    specs.extend(rows("tiff", TIFF_CMDS));
    specs
}
