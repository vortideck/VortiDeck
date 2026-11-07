#!/usr/bin/env node
// Check that JS Tauri plugin versions in pnpm.overrides match Rust Cargo.toml versions
// Run: pnpm run check:tauri-versions

const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..');
const pkg = require(path.join(ROOT, 'package.json'));
const overrides = (pkg.pnpm && pkg.pnpm.overrides) || {};

// Plugins that don't have a @tauri-apps/plugin-* JS counterpart
const SKIP = new Set([
  'tauri-plugin-pinia',
  'tauri-plugin-single-instance',
  'tauri-plugin-keep-screen-on',
  'tauri-plugin-biometry',
  'tauri-plugin-better-posthog',
]);

const cargoFiles = [
  { file: 'apps/tauri-mobile/src-tauri/Cargo.toml', name: 'Mobile (Android/iOS)' },
  { file: 'apps/tauri-windows/src-tauri/Cargo.toml', name: 'Desktop (Windows)' },
];

let errors = 0;

for (const { file, name } of cargoFiles) {
  const cargoPath = path.join(ROOT, file);
  if (!fs.existsSync(cargoPath)) {
    console.log(`  SKIP ${file} not found`);
    continue;
  }

  console.log(`\nChecking ${name}...`);
  const content = fs.readFileSync(cargoPath, 'utf-8');

  for (const line of content.split('\n')) {
    const pluginMatch = line.match(/(tauri-plugin-[a-z-]+)/);
    const versionMatch = line.match(/"(\d+\.\d+[\d.]*?)"/);

    if (!pluginMatch || !versionMatch) continue;

    const crate = pluginMatch[1];
    const rustVersion = versionMatch[1];

    if (SKIP.has(crate)) continue;

    // tauri-plugin-http -> @tauri-apps/plugin-http
    const jsName = '@tauri-apps/' + crate.replace('tauri-', '');
    const overrideVersion = overrides[jsName];

    if (!overrideVersion) {
      console.log(`  \x1b[31mMISSING\x1b[0m ${jsName} not in pnpm.overrides (Rust: ${rustVersion})`);
      errors++;
    } else if (overrideVersion !== rustVersion) {
      console.log(`  \x1b[31mMISMATCH\x1b[0m ${jsName}: override=${overrideVersion} rust=${rustVersion}`);
      errors++;
    } else {
      console.log(`  \x1b[32mOK\x1b[0m ${jsName} ${overrideVersion}`);
    }
  }
}

console.log('');
if (errors > 0) {
  console.log(`\x1b[31mFound ${errors} version mismatch(es)!\x1b[0m`);
  console.log('Update pnpm.overrides in package.json to match Cargo.toml versions.');
  process.exit(1);
} else {
  console.log('\x1b[32mAll Tauri plugin versions match.\x1b[0m');
}
