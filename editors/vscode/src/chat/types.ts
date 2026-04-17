import * as vscode from "vscode";
import { LanguageClient } from "vscode-languageclient/node";

export interface CommandContext {
  request: vscode.ChatRequest;
  context: vscode.ChatContext;
  response: vscode.ChatResponseStream;
  token: vscode.CancellationToken;
  systemPrompt: string;
  client: LanguageClient;
}

export enum DiagnosticCategory {
  Error = "error",
  Security = "security",
  Taint = "taint",
  ThreadSafety = "thread_safety",
  ControlFlow = "control_flow",
  Performance = "performance",
  Style = "style",
  Optimiser = "optimiser",
  Irules = "irules",
}
