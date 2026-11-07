#!/usr/bin/env node
// Updates the public repository (github.com/vortideck/VortiDeck) from a revision of
// this repository, as one new commit on top of the public history.
//
//   node scripts/sync-public.mjs --public <public repo clone> [--rev HEAD] [--base <rev>]
//        [--message "Sync with VortiDeck 0.7.1"] [--author "Name <email>"] [--dry-run] [--push]
//
// Run it from a checkout of the branch the public code comes from (oss/prepare-public).
//
// 1. Checks that the public clone is clean and matches its origin.
// 2. Drift guard: exports the revision the public repo was last synced from (the
//    "Synced-From:" trailer of the latest sync commit, or --base for the first sync) and
//    compares it with the public repo. Any difference is a change made only in the public
//    repo, for example a merged pull request. The sync stops so those changes can be
//    brought into this repository first instead of being overwritten.
// 3. Exports --rev with scripts/export-public-snapshot.mjs (exclusions, link cleanup,
//    secret and personal-data scan). A likely secret stops the sync.
// 4. Commits the exported tree to the public clone with a "Synced-From: <sha>" trailer,
//    and pushes it with --push. --dry-run only prints what would change.

import { execFileSync, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const args = process.argv.slice(2);
const option = (name, fallback) => {
  const index = args.indexOf(name);
  return index >= 0 ? args[index + 1] : fallback;
};
const flag = (name) => args.includes(name);
const usage = 'Usage: node scripts/sync-public.mjs --public <public repo clone> [--rev HEAD] [--base <rev>] [--message "..."] [--author "Name <email>"] [--dry-run] [--push]';

const publicDir = option('--public') && path.resolve(option('--public'));
if (!publicDir) { console.error(usage); process.exit(2); }
const dryRun = flag('--dry-run');
const push = flag('--push');
if (dryRun && push) { console.error('--dry-run and --push cannot be used together.'); process.exit(2); }

const fail = (message) => { console.error(`\n${message}`); process.exit(1); };
const run = (cwd, command, extra = {}) => {
  const result = spawnSync('git', command, { cwd, encoding: 'utf8', maxBuffer: 256 * 1024 * 1024, ...extra });
  if (result.status !== 0) fail(`git ${command.join(' ')} failed in ${cwd}:\n${result.stderr || result.stdout}`);
  return result.stdout.trim();
};
const privateRoot = run(process.cwd(), ['rev-parse', '--show-toplevel']);
const resolve = (rev) => run(privateRoot, ['rev-parse', '--verify', `${rev}^{commit}`]);
const rev = resolve(option('--rev', 'HEAD'));
const exporter = path.join(privateRoot, 'scripts/export-public-snapshot.mjs');

// --- public clone --------------------------------------------------------------------
if (!fs.existsSync(path.join(publicDir, '.git'))) fail(`${publicDir} is not a git clone of the public repository.`);
if (run(publicDir, ['status', '--porcelain'])) fail(`The public clone has uncommitted changes. Commit or remove them first:\n${run(publicDir, ['status', '--short'])}`);
const branch = run(publicDir, ['rev-parse', '--abbrev-ref', 'HEAD']);
run(publicDir, ['fetch', '-q', 'origin', branch]);
const local = run(publicDir, ['rev-parse', 'HEAD']);
const remote = run(publicDir, ['rev-parse', `origin/${branch}`]);
if (local !== remote) {
  const behindOnly = spawnSync('git', ['merge-base', '--is-ancestor', local, remote], { cwd: publicDir }).status === 0;
  if (!behindOnly) fail(`The public clone's ${branch} has commits that are not on origin. Push or drop them first.`);
  run(publicDir, ['merge', '--ff-only', '-q', remote]);
  console.log(`Fast-forwarded the public clone to origin/${branch}.`);
}

// Author identity: never let the personal address into the public history.
const author = option('--author')
  || `${run(publicDir, ['config', 'user.name'])} <${run(publicDir, ['config', 'user.email'])}>`;
const identity = author.match(/^(.+?)\s*<([^>]+)>$/);
if (!identity) fail('Set --author "Name <email>" or user.name and user.email in the public clone.');
if (/papadi166@gmail\.com/i.test(identity[2])) fail('Refusing to commit to the public repository with the personal email address.');
const identityEnv = {
  ...process.env,
  GIT_AUTHOR_NAME: identity[1], GIT_AUTHOR_EMAIL: identity[2],
  GIT_COMMITTER_NAME: identity[1], GIT_COMMITTER_EMAIL: identity[2],
};

// --- helpers -----------------------------------------------------------------------
const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'vortideck-sync-'));
process.on('exit', () => fs.rmSync(scratch, { recursive: true, force: true }));

const exportRevision = (revision, label) => {
  const out = path.join(scratch, label);
  const result = spawnSync(process.execPath, [exporter, '--out', out, '--rev', revision, '--scan-only'],
    { cwd: privateRoot, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  if (result.status !== 0) fail(`Export of ${revision.slice(0, 8)} failed:\n${result.stdout}\n${result.stderr}`);
  return { out, log: result.stdout.trim() };
};

// The tree object a commit of `dir` would get in the public repository. Using git's own
// index applies the public repo's line-ending rules, so checkouts on Windows and Linux
// compare equal.
const publicGitDir = run(publicDir, ['rev-parse', '--absolute-git-dir']);
const treeOf = (dir, label) => {
  const env = { ...process.env, GIT_INDEX_FILE: path.join(scratch, `${label}.index`) };
  const git = ['--git-dir', publicGitDir, '--work-tree', dir];
  run(dir, [...git, 'add', '-A'], { env });
  return run(dir, [...git, 'write-tree'], { env });
};

const changedFiles = (fromTree, toTree) =>
  run(publicDir, ['diff', '--name-status', fromTree, toTree]).split('\n').filter(Boolean);

// --- drift guard -------------------------------------------------------------------
const trailerLog = run(publicDir, ['log', '--format=%H%x09%(trailers:key=Synced-From,valueonly,separator=%x20)']);
const lastSync = trailerLog.split('\n').map((line) => line.split('\t')).find(([, value]) => value?.trim());
let base = option('--base');
if (base) base = resolve(base);
else if (lastSync) base = resolve(lastSync[1].trim());
else fail('No earlier sync commit (with a "Synced-From:" trailer) was found in the public repository.\nFor the first sync, pass --base <revision of this repository that the public repo currently matches>.');

const publicTree = run(publicDir, ['rev-parse', 'HEAD^{tree}']);
const baseTree = treeOf(exportRevision(base, 'base').out, 'base');
if (baseTree !== publicTree) {
  const drift = changedFiles(baseTree, publicTree);
  console.error(`The public repository has changes that are not in ${base.slice(0, 8)}, the revision it was last synced from:`);
  for (const line of drift.slice(0, 50)) console.error(`  ${line}`);
  if (drift.length > 50) console.error(`  ... ${drift.length - 50} more`);
  fail('Bring those changes into this repository first (for example by applying the pull request here), then sync again.\nInspect them with: git -C <public clone> diff ' + `${baseTree} ${publicTree}`);
}
console.log(`Drift check passed: the public repository matches ${base.slice(0, 8)}.`);

// --- export and compare ------------------------------------------------------------
const next = exportRevision(rev, 'next');
console.log(next.log);
const nextTree = treeOf(next.out, 'next');
if (nextTree === publicTree) {
  console.log(`\nThe public repository is already up to date with ${rev.slice(0, 8)}. Nothing to do.`);
  process.exit(0);
}
const changes = changedFiles(publicTree, nextTree);
console.log(`\n${changes.length} file(s) change in the public repository:`);
for (const line of changes.slice(0, 100)) console.log(`  ${line}`);
if (changes.length > 100) console.log(`  ... ${changes.length - 100} more`);

if (dryRun) {
  console.log('\nDry run: nothing was committed.');
  process.exit(0);
}

// --- commit ------------------------------------------------------------------------
const described = spawnSync('git', ['describe', '--tags', '--exact-match', '--match', 'desktop-v*', rev], { cwd: privateRoot, encoding: 'utf8' });
const version = described.status === 0 ? described.stdout.trim().replace(/^desktop-v/, '') : '';
const subject = option('--message', version ? `Sync with VortiDeck ${version}` : `Sync with the latest VortiDeck source`);
const message = `${subject}\n\nSynced-From: ${rev}\n`;
const commit = execFileSync('git', ['commit-tree', nextTree, '-p', 'HEAD', '-F', '-'],
  { cwd: publicDir, input: message, encoding: 'utf8', env: identityEnv }).trim();
run(publicDir, ['merge', '--ff-only', '-q', commit]);
console.log(`\nCommitted ${commit.slice(0, 8)} "${subject}" to the public clone.`);

if (push) {
  run(publicDir, ['push', '-q', 'origin', `HEAD:${branch}`]);
  console.log(`Pushed to origin/${branch}.`);
} else {
  console.log('Not pushed. Review it, then run: git -C <public clone> push');
}
