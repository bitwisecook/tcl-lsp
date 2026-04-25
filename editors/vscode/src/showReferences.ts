import * as vscode from "vscode";

export interface JsonPosition {
  line: number;
  character: number;
}

export interface JsonLocation {
  uri: string;
  range: {
    start: JsonPosition;
    end: JsonPosition;
  };
}

// Convert the JSON-RPC shapes that the LSP server serialises into the
// ``vscode.Uri`` / ``vscode.Position`` / ``vscode.Location`` instances that
// the built-in ``editor.action.showReferences`` command validates against.
export function convertShowReferencesArgs(
  uriString: string,
  position: JsonPosition,
  locations: ReadonlyArray<JsonLocation>,
): {
  uri: vscode.Uri;
  position: vscode.Position;
  locations: vscode.Location[];
} {
  return {
    uri: vscode.Uri.parse(uriString),
    position: new vscode.Position(position.line, position.character),
    locations: locations.map(
      (loc) =>
        new vscode.Location(
          vscode.Uri.parse(loc.uri),
          new vscode.Range(
            loc.range.start.line,
            loc.range.start.character,
            loc.range.end.line,
            loc.range.end.character,
          ),
        ),
    ),
  };
}
