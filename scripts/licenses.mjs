import fs from "node:fs";
import path from "node:path";
const lock = JSON.parse(fs.readFileSync("package-lock.json", "utf8"));
const rows = [];
for (const location of Object.keys(lock.packages).filter((p) =>
  p.startsWith("node_modules/"),
)) {
  const filename = path.join(location, "package.json");
  if (!fs.existsSync(filename)) continue;
  const pkg = JSON.parse(fs.readFileSync(filename, "utf8"));
  rows.push({
    ecosystem: "npm",
    name: pkg.name,
    version: pkg.version,
    license:
      typeof pkg.license === "string"
        ? pkg.license
        : pkg.license?.type || "See package license",
  });
}
const rustLock = fs.readFileSync("src-tauri/Cargo.lock", "utf8");
const cargoHome =
  process.env.CARGO_HOME || path.join(process.env.USERPROFILE, ".cargo");
const registryRoot = path.join(cargoHome, "registry", "src");
const registries = fs.existsSync(registryRoot)
  ? fs.readdirSync(registryRoot).map((p) => path.join(registryRoot, p))
  : [];
for (const block of rustLock.split("[[package]]").slice(1)) {
  const name = block.match(/^name = "([^"]+)"/m)?.[1];
  const version = block.match(/^version = "([^"]+)"/m)?.[1];
  if (!name || !version || name === "filepilot") continue;
  const file = registries
    .map((root) => path.join(root, `${name}-${version}`, "Cargo.toml"))
    .find((f) => fs.existsSync(f));
  const license = file
    ? fs.readFileSync(file, "utf8").match(/^license = "([^"]+)"/m)?.[1]
    : "Not downloaded for Windows";
  rows.push({
    ecosystem: "cargo",
    name,
    version,
    license: license || "See crate license",
  });
}
rows.sort((a, b) => (a.ecosystem + a.name).localeCompare(b.ecosystem + b.name));
fs.mkdirSync("docs", { recursive: true });
fs.writeFileSync(
  "docs/THIRD_PARTY_LICENSES.md",
  "# Dependency license inventory\n\nGenerated from the installed package metadata and Cargo lockfile. Includes build and test dependencies; platform-only crates may not be downloaded locally. Original license texts and copyright notices in the upstream packages remain applicable. SQLite is public domain. This inventory is not a replacement for those licenses.\n\n| Ecosystem | Package | Version | Declared license |\n| --- | --- | --- | --- |\n" +
    rows
      .map(
        (r) =>
          `| ${r.ecosystem} | ${r.name} | ${r.version} | ${r.license.replaceAll("|", "/")} |`,
      )
      .join("\n") +
    "\n",
);
console.log(`Recorded ${rows.length} dependency licenses`);
