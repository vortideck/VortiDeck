#!/usr/bin/env node

/**
 * Downloads the WebAssembly toolchain used by `vortideck build` when the
 * package does not bundle it (for example in a source checkout).
 *
 *   node bin/fetch-toolchain.js        download for this machine and print the directory
 *
 * Releases are pinned and every download is checked against its SHA-256 hash.
 * Files are cached per user, so each version downloads only once.
 */

const crypto = require('crypto');
const fs = require('fs');
const https = require('https');
const os = require('os');
const path = require('path');
const zlib = require('zlib');

const EXTISM_VERSION = 'v1.5.1';
const BINARYEN_VERSION = 'version_125';

// Hashes of the release assets as published by extism/js-pdk and WebAssembly/binaryen.
const TARGETS = {
  'win32-x64': {
    extism: ['x86_64-windows', '30fc5802de2281e1063210eee93ff10ca821a0a60966d629e8166bb28d9687c1'],
    binaryen: ['x86_64-windows', '7d20f9f22ee1d1a195aecccb21390f8c599b5025219520c7c9f04673b39d2c20'],
  },
  'linux-x64': {
    extism: ['x86_64-linux', 'f6a4a3a8f16651a527a4643f95d229c854a04d66d4628cdc724a58c699b93c95'],
    binaryen: ['x86_64-linux', '7c3bc16599c8274a04d34a504fe4be2047884f900e0e2da2f6fb9cd667183be4'],
  },
  'darwin-x64': {
    extism: ['x86_64-macos', '704d0924af76a532721a71c467bc00ebe76932f54e4cccc5336324c1fcb991b4'],
    binaryen: ['x86_64-macos', '72a98df1bfb81dd1e241d2b022e15c72edbd49e29d84abc6a1129c7d083072dd'],
  },
  'darwin-arm64': {
    extism: ['aarch64-macos', '04795df03c3077a7dec5cd39a073886b06b93ed199008eb5c1019613c4fe88cb'],
    binaryen: ['arm64-macos', '28bab047c4ce845c5c1da111222ffee5fafcec0bbedd046ad8b3dcae0fd57076'],
  },
};

function cacheRoot() {
  if (process.env.VORTIDECK_TOOLCHAIN_DIR) return process.env.VORTIDECK_TOOLCHAIN_DIR;
  if (process.platform === 'win32') {
    return path.join(process.env.LOCALAPPDATA || path.join(os.homedir(), 'AppData', 'Local'), 'vortideck', 'toolchain');
  }
  if (process.platform === 'darwin') return path.join(os.homedir(), 'Library', 'Caches', 'vortideck', 'toolchain');
  return path.join(process.env.XDG_CACHE_HOME || path.join(os.homedir(), '.cache'), 'vortideck', 'toolchain');
}

function download(url, redirects = 5) {
  return new Promise((resolve, reject) => {
    https.get(url, { headers: { 'User-Agent': 'vortideck-plugin-sdk' } }, (response) => {
      if ([301, 302, 303, 307, 308].includes(response.statusCode) && response.headers.location && redirects > 0) {
        response.resume();
        resolve(download(new URL(response.headers.location, url).toString(), redirects - 1));
        return;
      }
      if (response.statusCode !== 200) {
        response.resume();
        reject(new Error(`Download failed (${response.statusCode}): ${url}`));
        return;
      }
      const chunks = [];
      response.on('data', (chunk) => chunks.push(chunk));
      response.on('end', () => resolve(Buffer.concat(chunks)));
      response.on('error', reject);
    }).on('error', reject);
  });
}

async function verifiedDownload(url, sha256) {
  console.error(`Downloading ${url}`);
  const data = await download(url);
  const actual = crypto.createHash('sha256').update(data).digest('hex');
  if (actual !== sha256) throw new Error(`Checksum mismatch for ${url}: expected ${sha256}, got ${actual}`);
  return data;
}

// Minimal tar reader: regular files, symlinks, and GNU/pax long names.
function untar(buffer, onEntry) {
  let offset = 0;
  let longName = null;
  const text = (start, length) => buffer.toString('utf8', start, start + length).replace(/\0.*$/s, '');
  while (offset + 512 <= buffer.length) {
    const header = buffer.subarray(offset, offset + 512);
    if (header.every((byte) => byte === 0)) break;
    const size = parseInt(text(offset + 124, 12).trim() || '0', 8);
    const type = String.fromCharCode(header[156] || 48);
    const prefix = text(offset + 345, 155);
    const name = longName || (prefix ? `${prefix}/${text(offset, 100)}` : text(offset, 100));
    const body = buffer.subarray(offset + 512, offset + 512 + size);
    longName = null;
    if (type === 'L') {
      longName = body.toString('utf8').replace(/\0.*$/s, '');
    } else if (type === 'x') {
      const match = /\d+ path=([^\n]*)\n/.exec(body.toString('utf8'));
      if (match) longName = match[1];
    } else if (type === '0' || type === '2') {
      onEntry({ name, type, body, link: text(offset + 157, 100) });
    }
    offset += 512 + Math.ceil(size / 512) * 512;
  }
}

async function fetchExtism(dir, [asset, sha256]) {
  const file = path.join(dir, process.platform === 'win32' ? 'extism-js.exe' : 'extism-js');
  if (fs.existsSync(file)) return file;
  const url = `https://github.com/extism/js-pdk/releases/download/${EXTISM_VERSION}/extism-js-${asset}-${EXTISM_VERSION}.gz`;
  const binary = zlib.gunzipSync(await verifiedDownload(url, sha256));
  fs.writeFileSync(`${file}.partial`, binary, { mode: 0o755 });
  fs.renameSync(`${file}.partial`, file);
  return file;
}

async function fetchBinaryen(dir, [asset, sha256]) {
  const binDir = path.join(dir, 'binaryen', 'bin');
  const done = path.join(dir, 'binaryen', '.complete');
  if (fs.existsSync(done)) return binDir;
  const url = `https://github.com/WebAssembly/binaryen/releases/download/${BINARYEN_VERSION}/binaryen-${BINARYEN_VERSION}-${asset}.tar.gz`;
  const archive = zlib.gunzipSync(await verifiedDownload(url, sha256));
  const links = [];
  untar(archive, ({ name, type, body, link }) => {
    // Keep only the tools extism-js calls and the shared library they load on macOS.
    const relative = name.split('/').slice(1).join('/');
    const wanted = /^bin\/wasm-(opt|merge)(\.exe)?$/.test(relative) || /^lib\/libbinaryen[^/]*$/.test(relative);
    if (!wanted) return;
    const target = path.join(dir, 'binaryen', relative);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    if (type === '2') links.push([target, path.join(path.dirname(target), link)]);
    else fs.writeFileSync(target, body, { mode: 0o755 });
  });
  for (const [target, source] of links) {
    if (fs.existsSync(source)) fs.copyFileSync(source, target);
  }
  fs.writeFileSync(done, BINARYEN_VERSION);
  return binDir;
}

async function fetchToolchain() {
  const key = `${process.platform}-${process.arch}`;
  const target = TARGETS[key];
  if (!target) throw new Error(`No VortiDeck toolchain download for ${key}. Supported: ${Object.keys(TARGETS).join(', ')}.`);
  const dir = path.join(cacheRoot(), `extism-js-${EXTISM_VERSION}_binaryen-${BINARYEN_VERSION}`, key);
  fs.mkdirSync(dir, { recursive: true });
  const extismPath = await fetchExtism(dir, target.extism);
  const binaryenPath = await fetchBinaryen(dir, target.binaryen);
  return { extismPath, binaryenPath };
}

module.exports = { fetchToolchain };

if (require.main === module) {
  fetchToolchain()
    .then((toolchain) => process.stdout.write(JSON.stringify(toolchain)))
    .catch((error) => {
      console.error(`\n❌ ${error.message}`);
      process.exit(1);
    });
}
