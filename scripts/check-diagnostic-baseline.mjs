import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, relative, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const baselineDir = resolve(repoRoot, 'scripts', 'diagnostic-baselines')
const update = process.argv.includes('--update')
const selected = process.argv.slice(2).find(argument => !argument.startsWith('--'))

const definitions = {
  'desktop-lint': {
    baseline: 'desktop-lint.json',
    run: collectDesktopLint,
  },
  'desktop-typecheck': {
    baseline: 'desktop-typecheck.json',
    run: collectDesktopTypecheck,
  },
  'core-clippy': {
    baseline: 'core-clippy.json',
    run: collectCoreClippy,
  },
}

if (!selected || !definitions[selected]) {
  console.error(`Usage: node scripts/check-diagnostic-baseline.mjs <${Object.keys(definitions).join('|')}> [--update]`)
  process.exit(2)
}

const definition = definitions[selected]
const result = definition.run()
if (result.infrastructureError) {
  console.error(result.infrastructureError)
  process.exit(2)
}

const counts = countByKey(result.diagnostics)
const baselinePath = resolve(baselineDir, definition.baseline)

if (update) {
  mkdirSync(baselineDir, { recursive: true })
  const baseline = {
    schemaVersion: 1,
    gate: selected,
    total: result.diagnostics.length,
    byKey: Object.fromEntries([...counts].sort(([left], [right]) => left.localeCompare(right))),
  }
  writeFileSync(baselinePath, `${JSON.stringify(baseline, null, 2)}\n`, 'utf8')
  console.log(`Updated ${selected} baseline: ${baseline.total} diagnostic(s).`)
  process.exit(0)
}

if (!existsSync(baselinePath)) {
  console.error(`Missing diagnostic baseline: ${relative(repoRoot, baselinePath)}`)
  process.exit(2)
}

const baseline = JSON.parse(readFileSync(baselinePath, 'utf8'))
if (baseline.schemaVersion !== 1 || baseline.gate !== selected || typeof baseline.byKey !== 'object') {
  console.error(`Invalid diagnostic baseline: ${relative(repoRoot, baselinePath)}`)
  process.exit(2)
}

const regressions = []
for (const [key, count] of counts) {
  const allowed = baseline.byKey[key] ?? 0
  if (count > allowed) {
    regressions.push({ key, count, allowed })
  }
}

if (regressions.length > 0) {
  console.error(`${selected} introduced ${regressions.length} diagnostic bucket regression(s):`)
  for (const regression of regressions.slice(0, 30)) {
    console.error(`  ${regression.key}: ${regression.count} (baseline ${regression.allowed})`)
  }
  if (regressions.length > 30) {
    console.error(`  ...and ${regressions.length - 30} more`)
  }
  console.error('Fix the regression. Update the baseline only for an explicitly reviewed debt reclassification.')
  process.exit(1)
}

const reduced = baseline.total - result.diagnostics.length
console.log(
  `${selected} baseline passed: ${result.diagnostics.length}/${baseline.total} diagnostic(s)`
  + (reduced > 0 ? ` (${reduced} removed)` : ''),
)

function command(name, args, cwd = repoRoot) {
  const execution = spawnSync(name, args, {
    cwd,
    encoding: 'utf8',
    maxBuffer: 128 * 1024 * 1024,
    env: process.env,
    windowsHide: true,
  })

  if (execution.error) {
    return { infrastructureError: `Could not start ${name}: ${execution.error.message}` }
  }

  return {
    status: execution.status,
    stdout: execution.stdout ?? '',
    stderr: execution.stderr ?? '',
  }
}

function collectDesktopLint() {
  const execution = command(process.execPath, [
    resolve(repoRoot, 'node_modules', 'eslint', 'bin', 'eslint.js'),
    'apps/tauri-windows',
    'layers/base',
    'layers/configuration',
    'layers/desktop',
    'layers/desktop-streaming',
    'layers/flow',
    '--format',
    'json',
  ])
  if (execution.infrastructureError) return execution

  let reports
  try {
    reports = JSON.parse(execution.stdout)
  }
  catch (error) {
    return {
      infrastructureError: `ESLint did not produce JSON (exit ${execution.status}): ${error.message}\n${execution.stderr}`,
    }
  }

  const diagnostics = []
  for (const report of reports) {
    const file = normalizePath(report.filePath)
    for (const message of report.messages) {
      if (!message.ruleId || message.fatal) {
        return {
          infrastructureError: `ESLint configuration/parser failure in ${file}: ${message.message}`,
        }
      }
      diagnostics.push(`${file}|${message.severity === 2 ? 'error' : 'warning'}|${message.ruleId}`)
    }
  }

  if (execution.status !== 0 && diagnostics.length === 0) {
    return { infrastructureError: `ESLint exited ${execution.status} without a parsed diagnostic.` }
  }
  return { diagnostics }
}

function collectDesktopTypecheck() {
  const desktopRoot = resolve(repoRoot, 'apps', 'tauri-windows')
  const execution = command(
    process.execPath,
    [resolve(desktopRoot, 'node_modules', 'nuxt', 'bin', 'nuxt.mjs'), 'typecheck'],
    desktopRoot,
  )
  if (execution.infrastructureError) return execution

  const diagnostics = []
  const output = stripAnsi(`${execution.stdout}\n${execution.stderr}`)
  const pattern = /^(.+?)\((\d+),(\d+)\): error (TS\d+):/gm
  for (const match of output.matchAll(pattern)) {
    const absolute = resolve(repoRoot, 'apps', 'tauri-windows', match[1])
    diagnostics.push(`${normalizePath(absolute)}|${match[4]}`)
  }

  if (execution.status !== 0 && diagnostics.length === 0) {
    return {
      infrastructureError: `Desktop typecheck exited ${execution.status} without a parsed TypeScript diagnostic.\n${output.slice(-4000)}`,
    }
  }
  return { diagnostics }
}

function collectCoreClippy() {
  const execution = command('cargo', [
    'clippy',
    '--locked',
    '--manifest-path',
    'apps/tauri-windows/libs/vortideck-core/Cargo.toml',
    '--lib',
    '--tests',
    '--message-format=json',
    '--',
    '-D',
    'warnings',
  ])
  if (execution.infrastructureError) return execution

  const diagnostics = []
  for (const line of execution.stdout.split(/\r?\n/)) {
    if (!line.startsWith('{')) continue
    let record
    try {
      record = JSON.parse(line)
    }
    catch {
      continue
    }
    if (record.reason !== 'compiler-message' || record.message?.level !== 'error') continue

    const code = record.message.code?.code
    const span = record.message.spans?.find(candidate => candidate.is_primary)
    if (!code || !span?.file_name) {
      return {
        infrastructureError: `Clippy produced a non-baselineable compiler error: ${record.message.message}`,
      }
    }
    const absolute = resolve(repoRoot, 'apps', 'tauri-windows', 'libs', 'vortideck-core', span.file_name)
    diagnostics.push(`${normalizePath(absolute)}|${code}`)
  }

  if (execution.status !== 0 && diagnostics.length === 0) {
    return {
      infrastructureError: `Clippy exited ${execution.status} without a parsed lint diagnostic.\n${execution.stderr.slice(-4000)}`,
    }
  }
  return { diagnostics }
}

function normalizePath(path) {
  return relative(repoRoot, resolve(path)).replaceAll('\\', '/')
}

function stripAnsi(value) {
  return value.replace(/\x1B\[[0-?]*[ -/]*[@-~]/g, '')
}

function countByKey(diagnostics) {
  const counts = new Map()
  for (const diagnostic of diagnostics) {
    counts.set(diagnostic, (counts.get(diagnostic) ?? 0) + 1)
  }
  return counts
}
