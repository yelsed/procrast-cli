import * as esbuild from "esbuild";
import { resolve, dirname, join } from "path";
import { fileURLToPath } from "url";
import { existsSync, readFileSync, statSync } from "fs";

const __dirname = dirname(fileURLToPath(import.meta.url));
const nodeModules = resolve(__dirname, "node_modules");

function resolveFile(p) {
  if (!existsSync(p)) return null;
  if (statSync(p).isFile()) return p;

  // It's a directory — check for index files or package.json
  const pkgJson = join(p, "package.json");
  if (existsSync(pkgJson)) {
    const pkg = JSON.parse(readFileSync(pkgJson, "utf8"));
    const entry = pkg.module || pkg.main || "index.js";
    const resolved = join(p, entry);
    if (existsSync(resolved)) return resolved;
  }
  for (const idx of ["index.js", "index.mjs", "index.cjs"]) {
    const f = join(p, idx);
    if (existsSync(f)) return f;
  }
  return null;
}

const localResolvePlugin = {
  name: "local-resolve",
  setup(build) {
    build.onResolve({ filter: /^[^./]/ }, (args) => {
      const parts = args.path.split("/");
      const pkgName = parts[0].startsWith("@")
        ? parts.slice(0, 2).join("/")
        : parts[0];
      const subpath = parts.slice(pkgName.split("/").length).join("/");
      const pkgDir = join(nodeModules, pkgName);

      if (!existsSync(pkgDir)) return null;

      if (subpath) {
        const resolved = resolveFile(join(pkgDir, subpath));
        if (resolved) return { path: resolved };

        // Try via dist/esm
        const esmResolved = resolveFile(join(pkgDir, "dist", "esm", subpath));
        if (esmResolved) return { path: esmResolved };
      }

      // Root import — resolve via package.json
      const resolved = resolveFile(pkgDir);
      if (resolved) return { path: resolved };

      return null;
    });
  },
};

await esbuild.build({
  entryPoints: [resolve(__dirname, "src/index.ts")],
  bundle: true,
  platform: "node",
  format: "esm",
  target: "node18",
  outfile: resolve(__dirname, "dist/index.js"),
  banner: { js: "#!/usr/bin/env node" },
  plugins: [localResolvePlugin],
});

console.log("Built dist/index.js");
