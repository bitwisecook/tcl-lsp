import * as vscode from "vscode";
import { isTclLanguage } from "../extension";
import { CommandContext } from "./types";

/**
 * Extract the first fenced code block from LLM response text.
 * Looks for ```tcl or ``` blocks and returns the inner content.
 */
export function extractCodeBlock(text: string): string | undefined {
  const match = text.match(/```(?:tcl|irule)?\s*\n([\s\S]+?)```/);
  return match ? match[1].trimEnd() : undefined;
}

/**
 * Resolve iRule code from the chat request context.
 * Tries, in order: #file references, active editor selection, active editor content.
 */
export async function resolveIruleCode(ctx: CommandContext): Promise<string | undefined> {
  // 1. Check #file references
  for (const ref of ctx.request.references) {
    if (ref.value instanceof vscode.Uri) {
      const doc = await vscode.workspace.openTextDocument(ref.value);
      return doc.getText();
    }
    if (ref.value instanceof vscode.Location) {
      const doc = await vscode.workspace.openTextDocument(ref.value.uri);
      return doc.getText(ref.value.range);
    }
  }

  // 2. Active editor selection or full content
  const editor = vscode.window.activeTextEditor;
  if (editor && isTclLanguage(editor.document.languageId)) {
    if (!editor.selection.isEmpty) {
      return editor.document.getText(editor.selection);
    }
    return editor.document.getText();
  }

  // 3. Inline code block in prompt
  const extracted = extractCodeBlock(ctx.request.prompt);
  if (extracted) {
    return extracted;
  }

  return undefined;
}

/**
 * Resolve two code sources for comparison (used by /diff).
 * Tries: 2+ #file refs, 1 #file ref + active editor, or inline code blocks.
 */
export async function resolveTwoCodeSources(
  ctx: CommandContext,
): Promise<[string, string] | undefined> {
  const codes: string[] = [];

  // Collect from #file references
  for (const ref of ctx.request.references) {
    if (ref.value instanceof vscode.Uri) {
      const doc = await vscode.workspace.openTextDocument(ref.value);
      codes.push(doc.getText());
    } else if (ref.value instanceof vscode.Location) {
      const doc = await vscode.workspace.openTextDocument(ref.value.uri);
      codes.push(doc.getText(ref.value.range));
    }
    if (codes.length >= 2) {
      return [codes[0], codes[1]];
    }
  }

  // If we have one ref, try active editor as the other source
  if (codes.length === 1) {
    const editor = vscode.window.activeTextEditor;
    if (editor && isTclLanguage(editor.document.languageId)) {
      const editorCode = editor.selection.isEmpty
        ? editor.document.getText()
        : editor.document.getText(editor.selection);
      return [editorCode, codes[0]];
    }
  }

  // Try inline code blocks (two fenced blocks in the prompt)
  const blocks = [...ctx.request.prompt.matchAll(/```(?:tcl|irule)?\s*\n([\s\S]+?)```/g)];
  if (blocks.length >= 2) {
    return [blocks[0][1].trimEnd(), blocks[1][1].trimEnd()];
  }

  return undefined;
}

/**
 * Open an untitled Tcl document with the given content, or update an existing one.
 */
export async function ensureDocumentOpen(
  code: string,
  existingDoc?: vscode.TextDocument,
): Promise<vscode.TextDocument> {
  if (existingDoc) {
    const editor = await vscode.window.showTextDocument(existingDoc);
    const fullRange = new vscode.Range(0, 0, existingDoc.lineCount, 0);
    await editor.edit((edit) => edit.replace(fullRange, code));
    return existingDoc;
  }
  const doc = await vscode.workspace.openTextDocument({ language: "tcl", content: code });
  await vscode.window.showTextDocument(doc);
  return doc;
}
