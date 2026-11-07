import { existsSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'

const repositoryRoot = resolve(fileURLToPath(new URL('..', import.meta.url)))
const manifestPath = join(
  repositoryRoot,
  'apps',
  'tauri-windows',
  'src-tauri',
  'Cargo.toml',
)
const frontendDist = join(repositoryRoot, 'apps', 'tauri-windows', 'dist')
const rebuildFrontend = process.argv.includes('--frontend') || !existsSync(frontendDist)

function findCommand(command, args = ['--version']) {
  const result = spawnSync(command, args, {
    cwd: repositoryRoot,
    encoding: 'utf8',
    shell: false,
  })
  return result.status === 0 ? command : undefined
}

function run(command, args, env = process.env) {
  console.log(`\n> ${command} ${args.join(' ')}`)
  const result = spawnSync(command, args, {
    cwd: repositoryRoot,
    env,
    shell: process.platform === 'win32',
    stdio: 'inherit',
  })

  if (result.error) throw result.error
  if (result.status !== 0) process.exit(result.status ?? 1)
}

if (rebuildFrontend) {
  run('pnpm', ['generate:desktop'])
} else {
  console.log('Reusing apps/tauri-windows/dist (pass --frontend to regenerate it).')
}

const buildEnvironment = { ...process.env }
if (!buildEnvironment.CARGO_TARGET_DIR && buildEnvironment.VORTIDECK_CARGO_TARGET_DIR) {
  buildEnvironment.CARGO_TARGET_DIR = buildEnvironment.VORTIDECK_CARGO_TARGET_DIR
}
if (!buildEnvironment.CARGO_TARGET_DIR && process.platform === 'win32') {
  const localData = process.env.LOCALAPPDATA ?? process.env.USERPROFILE
  if (localData) {
    // Deep repository paths can make aws-lc-sys/CMake exceed MSBuild's path
    // limits. Keep one deterministic, user-local cache unless explicitly set.
    buildEnvironment.CARGO_TARGET_DIR = join(localData, 'VortiDeck', 'cargo-target')
  }
}
const sccache = findCommand('sccache')
if (sccache) {
  buildEnvironment.RUSTC_WRAPPER = sccache
  console.log('Rust compiler cache: sccache')
} else {
  console.log('Rust compiler cache: unavailable; continuing without it')
}

if (process.platform === 'win32' && process.env.VORTIDECK_USE_LLD !== '0') {
  const sysroot = spawnSync('rustc', ['--print', 'sysroot'], {
    cwd: repositoryRoot,
    encoding: 'utf8',
    shell: false,
  })
  const rustLld = sysroot.status === 0
    ? join(
        sysroot.stdout.trim(),
        'lib',
        'rustlib',
        'x86_64-pc-windows-msvc',
        'bin',
        'rust-lld.exe',
      )
    : undefined

  if (rustLld && existsSync(rustLld)) {
    buildEnvironment.CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = rustLld
    console.log(`Windows linker: ${rustLld}`)
  } else {
    console.log('Windows linker: rust-lld unavailable; using the toolchain default')
  }
}

if (buildEnvironment.CARGO_TARGET_DIR) {
  console.log(`Cargo target cache: ${buildEnvironment.CARGO_TARGET_DIR}`)
} else {
  console.log('Cargo target cache: Cargo default (no target-directory override)')
}

run(
  'cargo',
  [
    'build',
    '--locked',
    '--manifest-path',
    manifestPath,
    '--bin',
    'VortiDeck',
    '--features',
    'custom-protocol',
    '--profile',
    'inspection',
    '--timings',
  ],
  buildEnvironment,
)

if (sccache) run(sccache, ['--show-stats'])

console.log('\nInspection executable and Cargo timing report are in the active Cargo target directory.')
