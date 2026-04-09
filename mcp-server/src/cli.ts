import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { existsSync, mkdirSync, chmodSync, createWriteStream } from "node:fs";
import { resolve, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { homedir, tmpdir, platform, arch } from "node:os";
import { get as httpsGet } from "node:https";
import { pipeline } from "node:stream/promises";

const execFileAsync = promisify(execFile);

const __dirname = dirname(fileURLToPath(import.meta.url));

const BACKEND_URL = "https://procrastination-station-xwi7hrqi.on-forge.com";
const BINARY_NAMES = ["procrast", "procrast-cli"];
const CACHE_DIR = resolve(homedir(), ".cache", "procrast-cli", "bin");

let cliBinaryPath: string | null = null;

async function which(cmd: string): Promise<string | null> {
  try {
    const { stdout } = await execFileAsync("which", [cmd]);
    return stdout.trim() || null;
  } catch {
    return null;
  }
}

function httpsDownload(url: string, dest: string): Promise<void> {
  return new Promise((resolve, reject) => {
    httpsGet(url, { headers: { "User-Agent": "procrast-mcp-server" } }, (res) => {
      if (res.statusCode === 301 || res.statusCode === 302) {
        return httpsDownload(res.headers.location!, dest).then(resolve, reject);
      }
      if (res.statusCode !== 200) {
        return reject(new Error(`HTTP ${res.statusCode}`));
      }
      const stream = createWriteStream(dest);
      pipeline(res, stream).then(resolve, reject);
    }).on("error", reject);
  });
}

function getDownloadSlug(): string | null {
  const p = platform();
  const a = arch();
  if (p === "darwin" && a === "arm64") return "mac";
  if (p === "darwin" && a === "x64") return "mac-intel";
  if (p === "linux" && a === "x64") return "linux";
  return null;
}

async function downloadPrebuiltBinary(): Promise<string | null> {
  const slug = getDownloadSlug();
  if (!slug) return null;

  const binaryName = "procrast";
  const url = `${BACKEND_URL}/download/${slug}`;

  console.error(`[procrast-mcp] Downloading CLI binary from ${url}...`);

  try {
    const tmpFile = join(tmpdir(), `procrast-${slug}.tar.gz`);
    await httpsDownload(url, tmpFile);

    mkdirSync(CACHE_DIR, { recursive: true });
    const destBinary = join(CACHE_DIR, binaryName);

    await execFileAsync("tar", ["xzf", tmpFile, "-C", CACHE_DIR]);
    chmodSync(destBinary, 0o755);

    if (existsSync(destBinary)) {
      console.error(`[procrast-mcp] Installed CLI to ${destBinary}`);
      return destBinary;
    }
  } catch (e) {
    console.error(`[procrast-mcp] Download failed: ${e}`);
  }

  return null;
}

async function findCliBinary(): Promise<string> {
  if (cliBinaryPath) return cliBinaryPath;

  // 1. Check PATH (try both names)
  for (const name of BINARY_NAMES) {
    const onPath = await which(name);
    if (onPath) {
      cliBinaryPath = onPath;
      return onPath;
    }
  }

  // 2. Check ~/.cargo/bin/
  for (const name of BINARY_NAMES) {
    const cargoBin = resolve(homedir(), ".cargo", "bin", name);
    if (existsSync(cargoBin)) {
      cliBinaryPath = cargoBin;
      return cargoBin;
    }
  }

  // 3. Check cache dir (from previous download)
  for (const name of BINARY_NAMES) {
    const cached = join(CACHE_DIR, name);
    if (existsSync(cached)) {
      cliBinaryPath = cached;
      return cached;
    }
  }

  // 4. Download prebuilt binary from GitHub Releases
  const downloaded = await downloadPrebuiltBinary();
  if (downloaded) {
    cliBinaryPath = downloaded;
    return downloaded;
  }

  // 5. Fall back to cargo install (for users with Rust)
  const cargo = await which("cargo");
  if (cargo) {
    const cliRepoRoot = resolve(__dirname, "..", "..");
    const cargoToml = resolve(cliRepoRoot, "Cargo.toml");
    const installArgs = existsSync(cargoToml)
      ? ["install", "--path", cliRepoRoot]
      : ["install", "--git", "https://github.com/yelsed/procrast-cli"];

    console.error(`[procrast-mcp] No prebuilt binary available. Building from source...`);
    try {
      await execFileAsync("cargo", installArgs, { timeout: 300_000 });
      for (const name of BINARY_NAMES) {
        const installed = resolve(homedir(), ".cargo", "bin", name);
        if (existsSync(installed)) {
          cliBinaryPath = installed;
          return installed;
        }
      }
    } catch {
      // Build failed, fall through to error
    }
  }

  throw new Error(
    `procrast CLI not found. Install it with: curl -sSL ${BACKEND_URL}/install | sh`
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
    return 'Not authenticated. Run `procrast-cli login` in your terminal first.';
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
