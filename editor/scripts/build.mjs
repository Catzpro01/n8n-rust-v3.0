// SPDX-License-Identifier: AGPL-3.0-or-later

import { build } from "esbuild";
import { createHash } from "node:crypto";
import { copyFile, mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const editorRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const outputDirectory = join(editorRoot, "dist");
await rm(outputDirectory, { recursive: true, force: true });
await mkdir(outputDirectory, { recursive: true });

const entries = {
  main: join(editorRoot, "src/main.tsx"),
  benchmark: join(editorRoot, "src/benchmark-main.ts"),
};

const result = await build({
  entryPoints: entries,
  bundle: true,
  minify: true,
  format: "esm",
  target: ["chrome120", "edge120", "firefox121", "safari17"],
  outdir: join(outputDirectory, "assets"),
  entryNames: "[name]-[hash]",
  assetNames: "asset-[hash]",
  metafile: true,
  legalComments: "eof",
});

const outputs = Object.keys(result.metafile.outputs).map((path) =>
  relative(outputDirectory, path).replaceAll("\\", "/"),
);

function findByPrefix(prefix, suffix) {
  const found = outputs.find((p) => p.startsWith(`assets/${prefix}-`) && p.endsWith(suffix));
  if (!found) throw new Error(`missing ${prefix} ${suffix} in outputs: ${outputs.join(", ")}`);
  return found;
}

const script = findByPrefix("main", ".js");
const stylesheet = findByPrefix("main", ".css");
const benchScript = findByPrefix("benchmark", ".js");
const benchStylesheet = findByPrefix("benchmark", ".css");

const html = `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta name="color-scheme" content="dark">
    <meta name="description" content="An independent workspace for reliable personal automation.">
    <title>Canopy Workbench</title>
    <link rel="stylesheet" href="/${stylesheet}">
  </head>
  <body>
    <div id="app"></div>
    <noscript>Canopy Workbench needs JavaScript in this browser to open the editor.</noscript>
    <script type="module" src="/${script}"></script>
  </body>
</html>
`;
await writeFile(join(outputDirectory, "index.html"), html);

// Benchmark page with substituted asset paths.
const benchTemplate = await readFile(join(editorRoot, "bench.html"), "utf8");
const benchHtml = benchTemplate
  .replace("/dist/assets/benchmark.css", `/${benchStylesheet}`)
  .replace("/dist/assets/benchmark.js", `/${benchScript}`);
await writeFile(join(outputDirectory, "bench.html"), benchHtml);

const contracts = await Promise.all([
  "manual-trigger.v1alpha1.json",
  "generate-items.v1alpha1.json",
  "edit-fields.v1alpha2.json",
  "if.v1alpha1.json",
  "merge.v1alpha1.json",
  "summarize.v1alpha1.json",
].map(async (name) => JSON.parse(
  await readFile(join(editorRoot, "../contracts", name), "utf8"),
)));
const catalogNodes = contracts.map((contract) => {
  const contractBytes = JSON.stringify(canonical(contract));
  return {
    display_name: contract.extensions["canopy.workbench/display"].display_name,
    description: contract.extensions["canopy.workbench/display"].description,
    contract_lock: {
      api_version: contract.identity.api_version,
      namespace: contract.identity.namespace,
      name: contract.identity.name,
      version: contract.identity.version,
      digest: `sha256:${createHash("sha256").update(contractBytes).digest("hex")}`,
    },
    configuration_schema: contract.configuration.schema,
    editor_hints: contract.configuration.editor_hints,
  };
});
await writeFile(
  join(outputDirectory, "catalog.v1.json"),
  JSON.stringify({ schema: 1, nodes: catalogNodes }),
);

// Symlink/copy the generated fixtures directory so the benchmark page can
// fetch /tests/fixtures/...cwbt when served from a static host (used by
// Playwright CI runs that don't start workflowd).
const fixturesOut = join(outputDirectory, "tests", "fixtures");
await mkdir(fixturesOut, { recursive: true });

// Copy the 100k fixture (if present) so the /bench.html page can load it
// from the same origin without requiring workflowd to be running.
const fixtureSrc = join(editorRoot, "tests", "fixtures");
const fixtureDst = join(outputDirectory, "tests", "fixtures");
await mkdir(fixtureDst, { recursive: true });
for (const name of ["eco-100k-editor-fixture.cwbt", "eco-100k-editor-fixture.manifest.json"]) {
  try {
    await copyFile(join(fixtureSrc, name), join(fixtureDst, name));
  } catch {
    // Fixture is generated on demand (`npm run generate:fixture`); build
    // must still succeed without it so embedded-asset generation works.
  }
}

const files = await listFiles(outputDirectory);
const manifest = {
  schema: 1,
  generatedBy: "editor/scripts/build.mjs",
  entrypoints: {
    index: "/index.html",
    benchmark: "/bench.html",
    script: `/${script}`,
    stylesheet: `/${stylesheet}`,
    benchScript: `/${benchScript}`,
    benchStylesheet: `/${benchStylesheet}`,
  },
  files: await Promise.all(
    files.map(async (path) => {
      const bytes = await readFile(path);
      return {
        path: relative(outputDirectory, path).replaceAll("\\", "/"),
        bytes: bytes.length,
        sha256: createHash("sha256").update(bytes).digest("hex"),
      };
    }),
  ),
};
await writeFile(
  join(outputDirectory, "asset-manifest.json"),
  `${JSON.stringify(manifest, null, 2)}\n`,
);

async function listFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const paths = [];
  for (const entry of entries) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) paths.push(...(await listFiles(path)));
    else if (entry.isFile()) paths.push(path);
  }
  return paths.sort();
}

function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical);
  if (value !== null && typeof value === "object") {
    return Object.fromEntries(Object.keys(value).sort().map((key) => [key, canonical(value[key])]));
  }
  return value;
}
