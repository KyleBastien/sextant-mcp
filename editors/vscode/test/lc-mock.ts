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

export type ServerOptions = unknown;
export type LanguageClientOptions = unknown;

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
