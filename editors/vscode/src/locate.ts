import { execFileSync } from "child_process";
import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";

const BINARY = process.platform === "win32" ? "sextant-lsp.exe" : "sextant-lsp";

/**
 * Resolve the sextant-lsp binary path. Honours `sextant.serverPath` first,
 * otherwise looks the binary up via PATH (`where`/`which`). Returns
 * `undefined` when nothing is found; the caller is responsible for
 * surfacing an install hint.
 */
export function locateServer(): string | undefined {
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
