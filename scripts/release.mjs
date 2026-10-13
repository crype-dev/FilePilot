import fs from "node:fs";
import path from "node:path";
const mode = process.argv[2];
const pkg = JSON.parse(fs.readFileSync("package.json", "utf8"));
const config = JSON.parse(fs.readFileSync("src-tauri/tauri.conf.json", "utf8"));
const cargo = fs
  .readFileSync("src-tauri/Cargo.toml", "utf8")
  .match(/^version = "([^"]+)"/m)?.[1];
if (
  pkg.version !== config.version ||
  cargo !== pkg.version ||
  !/^\d+\.\d+\.\d+$/.test(pkg.version)
)
  throw new Error("Stable versions must agree");
if (
  process.env.GITHUB_REF_TYPE === "tag" && process.env.GITHUB_REF_NAME &&
  process.env.GITHUB_REF_NAME !== `v${pkg.version}`
)
  throw new Error("Tag does not match application version");
if (mode === "configure") {
  const pubkey = process.env.TAURI_UPDATER_PUBLIC_KEY;
  if (!pubkey || !process.env.TAURI_SIGNING_PRIVATE_KEY)
    throw new Error("Production signing keys are missing; release blocked");
  fs.mkdirSync("work", { recursive: true });
  fs.writeFileSync(
    "work/release.json",
    JSON.stringify({
      bundle: { createUpdaterArtifacts: true },
      plugins: {
        updater: {
          pubkey,
          requireSignedVersion: true,
          endpoints: [
            "https://github.com/crype-dev/FilePilot/releases/latest/download/latest.json",
          ],
          windows: { installMode: "passive" },
        },
      },
    }),
  );
} else if (mode === "manifest") {
  const folder = "src-tauri/target/release/bundle/nsis";
  const installers = fs.readdirSync(folder).filter((f) => f.endsWith(".exe"));
  if (installers.length !== 1)
    throw new Error("Expected exactly one NSIS installer");
  const installer = installers[0];
  const signature = fs
    .readFileSync(path.join(folder, installer + ".sig"), "utf8")
    .trim();
  if (!signature || fs.statSync(path.join(folder, installer)).size < 100000)
    throw new Error("Missing signature or invalid installer");
  const name = `FilePilot_${pkg.version}_x64-setup.exe`;
  fs.mkdirSync("work/release", { recursive: true });
  fs.copyFileSync(path.join(folder, installer), "work/release/" + name);
  fs.copyFileSync(
    path.join(folder, installer + ".sig"),
    "work/release/" + name + ".sig",
  );
  const manifest = {
    version: pkg.version,
    notes: `See CHANGELOG.md for FilePilot ${pkg.version}.`,
    pub_date: new Date().toISOString(),
    platforms: {
      "windows-x86_64": {
        signature,
        url: `https://github.com/crype-dev/FilePilot/releases/download/v${pkg.version}/${name}`,
      },
    },
  };
  fs.writeFileSync(
    "work/release/latest.json",
    JSON.stringify(manifest, null, 2),
  );
} else if (mode !== "check")
  throw new Error("Usage: release.mjs check|configure|manifest");
