import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { existsSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { homedir } from "node:os";

const execFileAsync = promisify(execFile);

const __dirname = dirname(fileURLToPath(import.meta.url));

let cliBinaryPath: string | null = null;

async function which(cmd: string): Promise<string | null> {
  try {
    const { stdout } = await execFileAsync("which", [cmd]);
    return stdout.trim() || null;
  } catch {
    return null;
  }
}

async function findCliBinary(): Promise<string> {
  if (cliBinaryPath) return cliBinaryPath;

  // 1. Check PATH (try both names)
  for (const name of ["procrast", "procrast-cli"]) {
    const onPath = await which(name);
    if (onPath) {
      cliBinaryPath = onPath;
      return onPath;
    }
  }

  // 2. Check ~/.cargo/bin/
  for (const name of ["procrast", "procrast-cli"]) {
    const cargoBin = resolve(homedir(), ".cargo", "bin", name);
    if (existsSync(cargoBin)) {
      cliBinaryPath = cargoBin;
      return cargoBin;
    }
  }

  // 3. Auto-install via cargo
  const cargo = await which("cargo");
  if (cargo) {
    // Try local repo first (for development), then GitHub
    const cliRepoRoot = resolve(__dirname, "..", "..");
    const cargoToml = resolve(cliRepoRoot, "Cargo.toml");
    const installArgs = existsSync(cargoToml)
      ? ["install", "--path", cliRepoRoot]
      : ["install", "--git", "https://github.com/yelsed/procrast-cli"];

    console.error(`[procrast-mcp] CLI not found. Installing...`);
    try {
      await execFileAsync("cargo", installArgs, { timeout: 300_000 });
      for (const name of ["procrast", "procrast-cli"]) {
        const installed = resolve(homedir(), ".cargo", "bin", name);
        if (existsSync(installed)) {
          cliBinaryPath = installed;
          return installed;
        }
      }
    } catch {
      // Installation failed, fall through to error
    }
  }

  throw new Error(
    "procrast CLI not found. Install Rust (https://rustup.rs) then run: cargo install --git https://github.com/yelsed/procrast-cli"
  );
}

export interface CliResult {
  stdout: string;
  stderr: string;
  exitCode: number;
}

export async function execCli(args: string[]): Promise<CliResult> {
  const binary = await findCliBinary();

  try {
    const { stdout, stderr } = await execFileAsync(binary, args, {
      timeout: 30_000,
      env: { ...process.env, NO_COLOR: "1" },
    });
    return { stdout, stderr, exitCode: 0 };
  } catch (err: unknown) {
    const e = err as {
      stdout?: string;
      stderr?: string;
      code?: number | string;
    };
    if (e.code === "ETIMEDOUT" || e.code === "ERR_CHILD_PROCESS_STDIO_MAXBUFFER") {
      throw new Error("CLI command timed out. The Procrast API may be unreachable.");
    }
    return {
      stdout: e.stdout ?? "",
      stderr: e.stderr ?? "",
      exitCode: typeof e.code === "number" ? e.code : 1,
    };
  }
}

export function parseAuthError(result: CliResult): string | null {
  const combined = result.stderr + result.stdout;
  if (
    combined.includes("Not logged in") ||
    combined.includes("Session expired")
  ) {
    return 'Not authenticated. Run `procrast login` in your terminal first.';
  }
  return null;
}

export function parseJsonOutput<T>(result: CliResult): T {
  const authErr = parseAuthError(result);
  if (authErr) throw new Error(authErr);

  if (result.exitCode !== 0) {
    throw new Error(result.stderr.trim() || "CLI command failed");
  }

  return JSON.parse(result.stdout) as T;
}
