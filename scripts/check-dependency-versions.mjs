import { readdirSync, readFileSync } from 'node:fs'
import { join, relative } from 'node:path'

const root = process.cwd()
const dependencySections = [
  'dependencies',
  'devDependencies',
  'optionalDependencies',
  'peerDependencies',
]
const ignoredDirectories = new Set([
  '.git',
  '.nuxt',
  '.output',
  'dist',
  'node_modules',
  'target',
])

// These are separate toolchains, not accidental version drift.
const allowedDifferences = new Map([
  [
    'tailwindcss',
    'The Nuxt advertiser still uses Tailwind 3; the isolated React landing experiment uses Tailwind 4.',
  ],
  [
    'typescript',
    'Workspace manifests request TypeScript 7 while the root maps it to the TypeScript 7 native bridge.',
  ],
])

function findManifests(directory) {
  const manifests = []

  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    if (entry.isDirectory() && ignoredDirectories.has(entry.name)) continue

    const path = join(directory, entry.name)
    if (entry.isDirectory()) manifests.push(...findManifests(path))
    else if (entry.isFile() && entry.name === 'package.json') manifests.push(path)
  }

  return manifests
}

const declarations = new Map()

for (const manifestPath of findManifests(root)) {
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'))

  for (const section of dependencySections) {
    for (const [name, spec] of Object.entries(manifest[section] ?? {})) {
      const entries = declarations.get(name) ?? []
      entries.push({
        file: relative(root, manifestPath),
        section,
        spec,
      })
      declarations.set(name, entries)
    }
  }
}

const mismatches = [...declarations.entries()]
  .map(([name, entries]) => ({
    name,
    entries,
    specs: [...new Set(entries.map(entry => entry.spec))],
  }))
  .filter(({ specs }) => specs.length > 1)

const unexpected = mismatches.filter(({ name }) => !allowedDifferences.has(name))

for (const { name, specs } of mismatches) {
  const reason = allowedDifferences.get(name)
  const label = reason ? 'allowed' : 'mismatch'
  console.log(`${label}: ${name} -> ${specs.join(', ')}`)
  if (reason) console.log(`  ${reason}`)
}

if (unexpected.length > 0) {
  console.error(`Found ${unexpected.length} unexpected dependency version mismatch(es).`)
  process.exit(1)
}

console.log(`Dependency declarations are aligned (${allowedDifferences.size} reviewed exceptions).`)
