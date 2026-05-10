// Stand-in for `vscode-languageclient/node` during vitest. The real
// module pulls in `vscode` transitively, which only exists inside the
// extension host. Tests don't need a working client — just the constructor
// surface so `buildClient` can return something `dispose()`-able.

import { vi } from "vitest";

export const TransportKind = { stdio: 0 } as const;

export class LanguageClient {
  readonly start = vi.fn(async () => {});
  readonly stop = vi.fn(async () => {});
  readonly dispose = vi.fn(() => {});
}

// Structural shapes matching how `extension.ts` populates these
// values. We don't need full fidelity with the real types from
// `vscode-languageclient` — the runtime here is the mock above.
export interface ServerOptions {
  readonly command: string;
  readonly args?: readonly string[];
  readonly transport?: number;
}

export interface DocumentFilter {
  readonly scheme: string;
  readonly language: string;
}

export interface LanguageClientOptions {
  readonly documentSelector?: readonly DocumentFilter[];
  readonly initializationOptions?: Readonly<Record<string, boolean>>;
  readonly synchronize?: {
    readonly fileEvents?: { dispose(): void };
    readonly configurationSection?: string;
  };
  readonly outputChannelName?: string;
}

if (typeof describe === "function") {
  describe("LanguageClient mock", () => {
    it("exposes start/stop as awaitable spies", async () => {
      const c = new LanguageClient();
      await expect(c.start()).resolves.toBeUndefined();
      await expect(c.stop()).resolves.toBeUndefined();
      expect(c.start).toHaveBeenCalledOnce();
      expect(c.stop).toHaveBeenCalledOnce();
    });
  });
}
