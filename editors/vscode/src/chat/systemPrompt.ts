// Backward-compatible wrapper — prefer buildPrompt(dialect) from ./promptLoader.
import { buildPrompt } from "./promptLoader";

export function buildSystemPrompt(): string {
  return buildPrompt("f5-irules");
}
