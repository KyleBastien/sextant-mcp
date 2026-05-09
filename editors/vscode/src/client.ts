import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

const DOCUMENT_SELECTORS = [
  { scheme: "file", language: "rust" },
  { scheme: "file", language: "python" },
  { scheme: "file", language: "go" },
  { scheme: "file", language: "java" },
  { scheme: "file", language: "typescript" },
  { scheme: "file", language: "typescriptreact" },
  { scheme: "file", language: "javascript" },
  { scheme: "file", language: "javascriptreact" },
];

export function buildClient(serverPath: string): LanguageClient {
  const serverOptions: ServerOptions = {
    command: serverPath,
    args: ["--stdio"],
    transport: TransportKind.stdio,
  };

  const disableLlm = vscode.workspace
    .getConfiguration("sextant")
    .get<boolean>("disableLlm", true);

  const clientOptions: LanguageClientOptions = {
    documentSelector: DOCUMENT_SELECTORS,
    initializationOptions: { disableLlm },
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher(
        "**/.sextant/{config.toml,rules/**}",
      ),
      configurationSection: "sextant",
    },
    outputChannelName: "Sextant",
  };

  return new LanguageClient(
    "sextant",
    "Sextant",
    serverOptions,
    clientOptions,
  );
}
