import { readdirSync, readFileSync } from 'node:fs'
import { dirname, extname, isAbsolute, join, relative, resolve } from 'node:path'

const root = process.cwd()
const baseRoot = resolve(root, 'layers/base')
const sourceExtensions = new Set(['.js', '.mjs', '.ts', '.vue'])
const ignoredDirectories = new Set(['.nuxt', '.output', 'dist', 'node_modules'])
const hostAliases = ['@advertiser/', '@blog/', '@desktop/', '@desktopStreaming/', '@flow/', '@mobile/']
const importPattern = /(?:\bfrom\s*|\bimport\s*(?:\(\s*)?|\brequire\s*\(\s*)["']([^"']+)["']/g

function sourceFiles(directory) {
  const files = []
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    if (entry.isDirectory() && ignoredDirectories.has(entry.name)) continue
    const path = join(directory, entry.name)
    if (entry.isDirectory()) files.push(...sourceFiles(path))
    else if (entry.isFile() && sourceExtensions.has(extname(entry.name))) files.push(path)
  }
  return files
}

function isOutsideBase(specifier, file) {
  if (!specifier.startsWith('.')) return false
  const target = resolve(dirname(file), specifier.split(/[?#]/, 1)[0])
  const fromBase = relative(baseRoot, target)
  return fromBase === '..' || fromBase.startsWith(`..${process.platform === 'win32' ? '\\' : '/'}`) || isAbsolute(fromBase)
}

const violations = []
for (const file of sourceFiles(baseRoot)) {
  const source = readFileSync(file, 'utf8')
  for (const match of source.matchAll(importPattern)) {
    const specifier = match[1]
    const reason =
      specifier.startsWith('@/') || specifier.startsWith('~/')
        ? 'ambiguous app-root alias; use @base for base-owned code'
        : hostAliases.some(alias => specifier.startsWith(alias))
          ? 'base cannot import a host/domain layer alias'
          : isOutsideBase(specifier, file)
            ? 'relative import escapes layers/base'
            : null

    if (reason) {
      violations.push(`${relative(root, file)}: ${specifier} (${reason})`)
    }
  }
}

if (violations.length > 0) {
  console.error('Forbidden base-layer dependencies:')
  for (const violation of violations) console.error(`  ${violation}`)
  process.exit(1)
}

console.log('Base-layer import boundary is clean.')
