// Minimal `vscode` stand-in for vitest. Only the surface the extension
// touches at activate/deactivate time is mocked; tests should call
// `(workspace.getConfiguration as Mock).mockReturnValue(...)` to shape
// individual scenarios.

import { vi } from "vitest";

export const window = {
  showErrorMessage: vi.fn(),
};

export const workspace = {
  getConfiguration: vi.fn(),
  createFileSystemWatcher: vi.fn(() => ({})),
};

export type ExtensionContext = {
  subscriptions: { dispose(): void }[];
};
