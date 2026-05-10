import { execFileSync } from "child_process";
import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

const BINARY = process.platform === "win32" ? "sextant-lsp.exe" : "sextant-lsp";

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

let client: LanguageClient | undefined;

// Narrow context type capturing only what `activate` actually uses.
// VS Code at runtime always passes the full ExtensionContext (which
// satisfies this shape structurally), and tests can construct a value
// inline without `as`-casting through `unknown`.
type ActivateContext = Pick<vscode.ExtensionContext, "subscriptions">;

export async function activate(context: ActivateContext): Promise<void> {
  return activateWith(context, locateServer());
}

// Internal entry point with the binary lookup factored out so tests
// can drive the missing-binary path without mocking vscode.workspace.
async function activateWith(
  context: ActivateContext,
  serverPath: string | undefined,
): Promise<void> {
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

function locateServer(): string | undefined {
  const configured = vscode.workspace
    .getConfiguration("sextant")
    .get<string | null>("serverPath");
  if (configured && configured.length > 0) {
    if (fs.existsSync(configured)) {
      return configured;
    }
    return undefined;
  }
  return findOnPath(BINARY);
}

function findOnPath(name: string): string | undefined {
  const cmd = process.platform === "win32" ? "where" : "which";
  try {
    const out = execFileSync(cmd, [name], { encoding: "utf8" }).trim();
    const first = out.split(/\r?\n/)[0];
    if (first && fs.existsSync(first)) {
      return path.resolve(first);
    }
  } catch {
    // not found — fall through
  }
  return undefined;
}

function buildClient(serverPath: string): LanguageClient {
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

  return new LanguageClient("sextant", "Sextant", serverOptions, clientOptions);
}

// In-source tests. `globals: true` in vitest.config.ts makes
// describe/it/expect/vi/beforeEach available; outside vitest the
// `typeof` guard skips the block at runtime.
if (typeof describe === "function") {
  describe("extension lifecycle", () => {
    beforeEach(() => {
      client = undefined;
      vi.resetAllMocks();
    });

    it("activate surfaces an error and bails when the binary is missing", async () => {
      const ctx = { subscriptions: [] };
      await activateWith(ctx, undefined);
      expect(vscode.window.showErrorMessage).toHaveBeenCalledOnce();
      expect(ctx.subscriptions).toHaveLength(0);
    });

    it("deactivate is a no-op when activate never ran", async () => {
      await expect(deactivate()).resolves.toBeUndefined();
    });
  });
}
