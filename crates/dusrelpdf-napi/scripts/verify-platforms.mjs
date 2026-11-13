#!/usr/bin/env node
/**
 * Verify the napi-rs prebuild wiring is consistent. No publishing,
 * no network — it only reads files in this repo.
 *
 * Checks:
 * 1. Every triple in `crates/dusrelpdf-napi/package.json`
 *    (`napi.triples`) has a staged platform package at
 *    `crates/dusrelpdf-napi/npm/<platform>-<arch>-<abi>/` with a
 *    `package.json` whose `name` is `dusrelpdf-<triple>`, whose
 *    `version` matches, whose `main`/`files` point at the
 *    `<binary>.<triple>.node` file, and whose `os`/`cpu`/`libc`
 *    fields match the triple — the fields npm uses to pick the
 *    right optionalDependency at install time.
 * 2. `packages/js/package.json` `optionalDependencies` lists
 *    exactly those platform packages, each pinned to exactly
 *    this version (no `^` range).
 * 3. With `--staged`, each platform directory also contains the
 *    built `.node` file (non-empty) — i.e. what `napi artifacts`
 *    copied in. Without `--staged`, only the manifests are checked
 *    (the `.node` files are gitignored build outputs).
 *
 * Usage:
 *   node scripts/verify-platforms.mjs            # manifests only
 *   node scripts/verify-platforms.mjs --staged  # + staged .node files
 */
import { existsSync, readFileSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "..",
  "..",
);
const napiDir = path.join(repoRoot, "crates", "dusrelpdf-napi");
const jsDir = path.join(repoRoot, "packages", "js");

const staged = process.argv.includes("--staged");
const failures = [];
const fail = (msg) => failures.push(msg);

function readJson(file) {
  try {
    return JSON.parse(readFileSync(file, "utf8"));
  } catch (e) {
    fail(`cannot read ${path.relative(repoRoot, file)}: ${e.message}`);
    return null;
  }
}

/** `x86_64-unknown-linux-gnu` -> { platformArchABI: "linux-x64-gnu",
 *  os: "linux", cpu: "x64", libc: "glibc" } (libc null on darwin). */
function parseTriple(raw) {
  const parts = raw.split("-");
  const cpuRaw = parts[0];
  const sys = parts[2];
  const abi = parts[3] ?? null;
  const cpu = cpuRaw === "x86_64" ? "x64" : cpuRaw === "aarch64" ? "arm64" : cpuRaw;
  const os = sys === "windows" ? "win32" : sys;
  const platformArchABI = abi ? `${os}-${cpu}-${abi}` : `${os}-${cpu}`;
  const libc =
    abi === "gnu" ? "glibc" : abi === "musl" ? "musl" : null;
  return { platformArchABI, os, cpu, libc };
}

const napiPkg = readJson(path.join(napiDir, "package.json"));
const jsPkg = readJson(path.join(jsDir, "package.json"));
if (!napiPkg || !jsPkg) {
  console.error(failures.join("\n"));
  process.exit(1);
}

const version = napiPkg.version;
const binaryName = napiPkg.napi?.name;
const packageName = napiPkg.napi?.package?.name;
if (!binaryName || !packageName) {
  fail(
    "crates/dusrelpdf-napi/package.json needs napi.name (binary) and napi.package.name (platform package prefix)",
  );
}
const triples = [
  ...(napiPkg.napi?.triples?.additional ?? []),
  ...((napiPkg.napi?.triples?.defaults ?? true)
    ? ["x86_64-pc-windows-msvc", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu"]
    : []),
];
if (!triples.length) fail("no napi triples configured");

const expectedOptional = {};
for (const raw of triples) {
  const { platformArchABI, os, cpu, libc } = parseTriple(raw);
  const platformName = `${packageName}-${platformArchABI}`;
  const nodeFile = `${binaryName}.${platformArchABI}.node`;
  expectedOptional[platformName] = version;

  const dir = path.join(napiDir, "npm", platformArchABI);
  const manifestFile = path.join(dir, "package.json");
  if (!existsSync(manifestFile)) {
    fail(`missing platform package manifest: ${path.relative(repoRoot, manifestFile)}`);
    continue;
  }
  const manifest = readJson(manifestFile);
  if (!manifest) continue;
  const rel = path.relative(repoRoot, manifestFile);
  if (manifest.name !== platformName)
    fail(`${rel}: name ${JSON.stringify(manifest.name)} != ${JSON.stringify(platformName)}`);
  if (manifest.version !== version)
    fail(`${rel}: version ${JSON.stringify(manifest.version)} != ${JSON.stringify(version)}`);
  if (manifest.main !== nodeFile)
    fail(`${rel}: main ${JSON.stringify(manifest.main)} != ${JSON.stringify(nodeFile)}`);
  if (JSON.stringify(manifest.files) !== JSON.stringify([nodeFile]))
    fail(`${rel}: files ${JSON.stringify(manifest.files)} != ${JSON.stringify([nodeFile])}`);
  if (JSON.stringify(manifest.os) !== JSON.stringify([os]))
    fail(`${rel}: os ${JSON.stringify(manifest.os)} != ${JSON.stringify([os])}`);
  if (JSON.stringify(manifest.cpu) !== JSON.stringify([cpu]))
    fail(`${rel}: cpu ${JSON.stringify(manifest.cpu)} != ${JSON.stringify([cpu])}`);
  if (libc) {
    if (JSON.stringify(manifest.libc) !== JSON.stringify([libc]))
      fail(`${rel}: libc ${JSON.stringify(manifest.libc)} != ${JSON.stringify([libc])}`);
  }
  if (staged) {
    const nodePath = path.join(dir, nodeFile);
    if (!existsSync(nodePath)) {
      fail(`missing staged binary: ${path.relative(repoRoot, nodePath)} (run: napi artifacts)`);
    } else if (statSync(nodePath).size === 0) {
      fail(`staged binary is empty: ${path.relative(repoRoot, nodePath)}`);
    }
  }
}

const optional = jsPkg.optionalDependencies ?? {};
const optionalKeys = Object.keys(optional).sort();
const expectedKeys = Object.keys(expectedOptional).sort();
if (JSON.stringify(optionalKeys) !== JSON.stringify(expectedKeys)) {
  fail(
    `packages/js/package.json optionalDependencies keys ${JSON.stringify(optionalKeys)} != expected ${JSON.stringify(expectedKeys)}`,
  );
}
for (const [name, want] of Object.entries(expectedOptional)) {
  if (optional[name] !== undefined && optional[name] !== want) {
    fail(
      `packages/js/package.json optionalDependencies[${JSON.stringify(name)}] is ${JSON.stringify(optional[name])}, expected exact ${JSON.stringify(want)}`,
    );
  }
}

if (failures.length) {
  console.error(`verify-platforms: ${failures.length} problem(s):`);
  for (const f of failures) console.error(`  - ${f}`);
  process.exit(1);
}
console.log(
  `verify-platforms: OK — ${triples.length} platform packages ` +
    `(dusrelpdf-<triple> @ ${version}), optionalDependencies match` +
    (staged ? ", staged .node files present" : ""),
);
