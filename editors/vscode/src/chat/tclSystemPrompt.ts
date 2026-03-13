// Backward-compatible wrapper — prefer buildPrompt(dialect) from ./promptLoader.
import { buildPrompt } from "./promptLoader";

export function buildTclSystemPrompt(): string {
  return buildPrompt("tcl8.6");
}
