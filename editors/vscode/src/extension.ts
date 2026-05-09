import * as vscode from "vscode";
import { LanguageClient } from "vscode-languageclient/node";

import { buildClient } from "./client";
import { locateServer } from "./locate";

let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  const serverPath = locateServer();
  if (!serverPath) {
    vscode.window.showErrorMessage(
      "Sextant: could not find the sextant-lsp binary. Install with " +
        "`cargo install --path crates/sextant-lsp` or set " +
        "`sextant.serverPath` to its absolute path.",
    );
    return;
  }

  client = buildClient(serverPath);
  context.subscriptions.push(client);
  await client.start();
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
}
