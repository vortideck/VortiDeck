#!/usr/bin/env node
// Builds a clean public copy of the repository from a committed revision.
//
//   node scripts/export-public-snapshot.mjs --out <empty dir> [--rev HEAD] [--author "Name <email>"] [--scan-only]
//
// Steps: export the revision with `git archive` (committed files only), remove every
// path matched by scripts/public-export.exclude, scan the result for secrets and
// personal data, then create a new git repository with a single commit.
// Exits non-zero, without creating the commit, when the scan finds a likely secret.

import { execFileSync, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const root = execFileSync('git', ['rev-parse', '--show-toplevel'], { encoding: 'utf8' }).trim();
const args = process.argv.slice(2);
const option = (name, fallback) => {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : fallback;
};
const rev = option('--rev', 'HEAD');
const out = option('--out');
const author = option('--author');
const scanOnly = args.includes('--scan-only');
if (!out) {
  console.error('Usage: node scripts/export-public-snapshot.mjs --out <empty dir> [--rev HEAD] [--author "Name <email>"] [--scan-only]');
  process.exit(2);
}
const outDir = path.resolve(out);
if (fs.existsSync(outDir) && fs.readdirSync(outDir).length > 0) {
  console.error(`Refusing to write into a non-empty directory: ${outDir}`);
  process.exit(2);
}

// --- exclusion patterns -------------------------------------------------------
const toRegex = (pattern) => {
  const directory = pattern.endsWith('/');
  let body = '';
  const source = directory ? pattern.slice(0, -1) : pattern;
  for (let i = 0; i < source.length; i++) {
    const char = source[i];
    if (source.startsWith('**/', i)) { body += '(?:.*/)?'; i += 2; continue; }
    if (char === '*') { body += '[^/]*'; continue; }
    body += char.replace(/[.+?^${}()|[\]\\]/g, '\\$&');
  }
  return new RegExp(`^${body}${directory ? '(?:/.*)?' : ''}$`);
};
const patterns = fs.readFileSync(path.join(root, 'scripts/public-export.exclude'), 'utf8')
  .split(/\r?\n/).map((line) => line.trim()).filter((line) => line && !line.startsWith('#'))
  .map((line) => ({ line, regex: toRegex(line) }));

// --- export the committed revision -------------------------------------------
fs.mkdirSync(outDir, { recursive: true });
// A temporary index keeps the repository's real index untouched.
const tempIndex = path.join(outDir, '..', `.export-index-${process.pid}`);
const indexEnv = { ...process.env, GIT_INDEX_FILE: tempIndex };
for (const command of [['read-tree', rev], ['checkout-index', '--all', `--prefix=${outDir.replace(/\\/g, '/')}/`]]) {
  const result = spawnSync('git', command, { cwd: root, env: indexEnv, encoding: 'utf8' });
  if (result.status !== 0) { console.error(result.stderr); fs.rmSync(tempIndex, { force: true }); process.exit(1); }
}
fs.rmSync(tempIndex, { force: true });

const walk = (dir, base = '') => fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
  const relative = base ? `${base}/${entry.name}` : entry.name;
  return entry.isDirectory() ? walk(path.join(dir, entry.name), relative) : [relative];
});
const removedPaths = [];
for (const file of walk(outDir)) {
  if (patterns.some(({ regex }) => regex.test(file))) {
    fs.rmSync(path.join(outDir, file));
    removedPaths.push(file);
  }
}
const removed = removedPaths.length;
const pruneEmpty = (dir) => {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory()) pruneEmpty(path.join(dir, entry.name));
  }
  if (dir !== outDir && fs.readdirSync(dir).length === 0) fs.rmdirSync(dir);
};
pruneEmpty(outDir);

// --- links to excluded files ---------------------------------------------------
// Private docs may link to notes that stay private. Keep the link text and drop the
// link, so the public docs have no dead links and the private docs keep theirs.
const removedSet = new Set(removedPaths);
const removedDirs = new Set(removedPaths.flatMap((file) => {
  const parts = file.split('/');
  return parts.slice(1).map((_, i) => parts.slice(0, i + 1).join('/'));
}));
const pointsToExcluded = (markdownFile, target) => {
  const bare = target.split('#')[0];
  if (!bare || /^(?:[a-z][a-z0-9+.-]*:|\/|#)/i.test(bare)) return false;
  let decoded = bare;
  try { decoded = decodeURIComponent(bare); } catch { /* keep the raw target */ }
  const resolved = path.posix.normalize(path.posix.join(path.posix.dirname(markdownFile), decoded)).replace(/\/$/, '');
  if (resolved.startsWith('..') || fs.existsSync(path.join(outDir, resolved))) return false;
  return removedSet.has(resolved) || removedDirs.has(resolved);
};
let unlinked = 0;
for (const file of walk(outDir).filter((name) => name.endsWith('.md'))) {
  const full = path.join(outDir, file);
  const original = fs.readFileSync(full, 'utf8');
  // Odd segments are fenced code blocks, which are left alone.
  const updated = original.split(/(```[\s\S]*?```)/).map((segment, index) => index % 2 ? segment
    : segment.replace(/\[([^\]\n]*)\]\(([^)\s]+)(\s+"[^"]*")?\)/g, (match, label, target) => {
      if (!pointsToExcluded(file, target)) return match;
      unlinked++;
      return label;
    })).join('');
  if (updated !== original) fs.writeFileSync(full, updated);
}

const files = walk(outDir);
console.log(`Exported ${rev}: ${files.length} files kept, ${removed} excluded, ${unlinked} links to excluded files unlinked.`);

// --- scan ------------------------------------------------------------------------
const secretRules = [
  ['private key block', /-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----/],
  ['Stripe secret key', /\b[sr]k_(?:live|test)_[A-Za-z0-9]{10,}/],
  ['GitHub token', /\b(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{30,}/],
  ['AWS access key', /\bAKIA[0-9A-Z]{16}\b/],
  ['Slack token', /\bxox[abprs]-[A-Za-z0-9-]{10,}/],
  ['OpenAI/Anthropic key', /\bsk-(?:ant-|proj-)?[A-Za-z0-9_-]{32,}/],
  ['Google OAuth client secret', /\bGOCSPX-[A-Za-z0-9_-]{20,}/],
  ['Google API key', /\bAIza[0-9A-Za-z_-]{35}\b/],
  ['Upstash/Redis token', /\bAT[A-Za-z0-9]{40,}\b/],
];
const privacyRules = [
  ['Windows user path', /C:[\\/]+Users[\\/]+alexispace/i],
  ['personal email', /papadi166@gmail\.com/i],
  ['home LAN address', /192\.168\.178\.\d+/],
];
const textExtensions = /\.(?:rs|ts|tsx|js|mjs|cjs|vue|json|jsonc|toml|ya?ml|md|txt|html|css|scss|ps1|sh|kts|gradle|kt|java|xml|properties|env|example|cfg|ini|lock)$/i;
const findings = { secret: [], privacy: [] };
for (const file of files) {
  if (!textExtensions.test(file) && !/(^|\/)\.[^/]+$/.test(file)) continue;
  const full = path.join(outDir, file);
  if (fs.statSync(full).size > 5 * 1024 * 1024) continue;
  const lines = fs.readFileSync(full, 'utf8').split(/\r?\n/);
  lines.forEach((text, index) => {
    for (const [label, rule] of secretRules) if (rule.test(text)) findings.secret.push(`${file}:${index + 1} ${label}`);
    for (const [label, rule] of privacyRules) if (rule.test(text)) findings.privacy.push(`${file}:${index + 1} ${label}`);
  });
}
const report = (title, list) => {
  console.log(`\n${title}: ${list.length}`);
  for (const item of list.slice(0, 200)) console.log(`  ${item}`);
  if (list.length > 200) console.log(`  ... ${list.length - 200} more`);
};
report('Likely secrets', findings.secret);
report('Personal data', findings.privacy);

if (findings.secret.length > 0) {
  console.error('\nLikely secrets found. Fix them in the source branch and export again. No commit was created.');
  process.exit(1);
}
if (scanOnly) process.exit(0);

// --- fresh repository -------------------------------------------------------------
// The author identity is also used as committer so no other address leaks into the commit.
const identity = author?.match(/^(.+?)\s*<([^>]+)>$/);
if (author && !identity) { console.error('--author must look like "Name <email>"'); process.exit(2); }
const env = identity
  ? { ...process.env, GIT_AUTHOR_NAME: identity[1], GIT_AUTHOR_EMAIL: identity[2], GIT_COMMITTER_NAME: identity[1], GIT_COMMITTER_EMAIL: identity[2] }
  : process.env;
const git = (...command) => execFileSync('git', command, { cwd: outDir, stdio: 'inherit', env });
git('init', '-q', '-b', 'main');
git('add', '-A');
git('commit', '-q', '-m', 'Initial public release of VortiDeck');
console.log(`\nCreated a fresh repository with one commit in ${outDir}`);
