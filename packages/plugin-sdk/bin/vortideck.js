#!/usr/bin/env node

/**
 * VortiDeck Plugin SDK CLI
 *
 * Handles building plugins with zero configuration
 */

const { execFileSync, execSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const esbuild = require('esbuild');

const command = process.argv[2];

function validatePackageId(packageId) {
  if (!/^[A-Za-z0-9][A-Za-z0-9._-]{1,127}$/.test(packageId)) {
    throw new Error(
      `Unsafe VortiDeck package id '${packageId}'. ` +
      'Use 2-128 letters, numbers, dots, underscores, or hyphens, starting with a letter or number. ' +
      'Colons belong in logical node IDs, not installable package IDs.'
    );
  }
}

if (command === 'build') {
  buildPlugin();
} else if (command === 'init') {
  initPlugin();
} else {
  console.log(`
VortiDeck Plugin SDK CLI

Usage:
  npx vortideck build    Build your plugin to WASM
  npx vortideck init     Create a new plugin project

Example:
  npm install @vortideck/plugin-sdk
  npx vortideck build
  `);
}

/**
 * Build the plugin
 */
async function buildPlugin() {
  try {
    console.log('🔨 Building VortiDeck plugin...\n');

    // Step 1: Type check
    console.log('📝 Type checking...');
    try {
      execSync('npx tsc --noEmit', { stdio: 'inherit' });
      console.log('✅ Type check passed\n');
    } catch (error) {
      console.error('❌ Type check failed');
      process.exit(1);
    }

    // Step 2: Bundle with esbuild
    console.log('📦 Bundling with esbuild...');
    const entryPoint = fs.existsSync('src/main.ts') ? 'src/main.ts' : 'src/index.ts';

    await esbuild.build({
      entryPoints: [entryPoint],
      outdir: 'dist',
      bundle: true,
      sourcemap: true,
      minify: false,
      format: 'cjs',
      target: ['es2020'],
      external: ['@extism/js-pdk'],
    });
    console.log('✅ Bundle created\n');

    // Step 2.5: Copy index.d.ts from SDK PDK to dist
    // The SDK's src/index.d.ts is in the parent directory of bin/
    const sdkDtsPath = path.join(__dirname, '..', 'src', 'index.d.ts');

    if (!fs.existsSync(sdkDtsPath)) {
      console.error(`\n❌ SDK index.d.ts not found: ${sdkDtsPath}`);
      console.error('Please reinstall @vortideck/plugin-sdk');
      process.exit(1);
    }

    fs.copyFileSync(sdkDtsPath, 'dist/index.d.ts');
    console.log('✅ Copied TypeScript definitions from SDK\n');

    // Step 3: Compile to WASM
    console.log('🔧 Compiling to WASM with extism-js...');
    const bundleFile = fs.existsSync('dist/main.js') ? 'dist/main.js' : 'dist/index.js';

    // Get plugin name from package.json or use default
    let pluginName = 'plugin';
    if (fs.existsSync('package.json')) {
      const pkg = JSON.parse(fs.readFileSync('package.json', 'utf8'));
      pluginName = pkg.name.replace('@', '').replace('/', '-');
    }

    const outputFile = `dist/${pluginName}.wasm`;

    // Detect platform and use bundled extism-js binary
    const platform = process.platform; // 'win32', 'darwin', 'linux'
    const arch = process.arch; // 'x64', 'arm64'

    let extismBinary;
    if (platform === 'win32' && arch === 'x64') {
      extismBinary = 'extism-js-x86_64-windows.exe';
    } else if (platform === 'darwin' && (arch === 'x64' || arch === 'arm64')) {
      extismBinary = arch === 'arm64' ? 'extism-js-aarch64-macos' : 'extism-js-x86_64-macos';
    } else if (platform === 'linux' && arch === 'x64') {
      extismBinary = 'extism-js-x86_64-linux';
    } else {
      console.error(`\n❌ The bundled SDK toolchain does not support ${platform}-${arch}.`);
      console.error('Supported hosts: Windows x64, macOS x64/ARM64, and Linux x64 (including WSL).');
      process.exit(1);
    }

    let extismPath = path.join(__dirname, 'extism-js', extismBinary);
    let downloadedBinaryenPath = null;

    // Source checkouts do not bundle the toolchain: download the pinned release once.
    if (!fs.existsSync(extismPath)) {
      try {
        const output = execFileSync(process.execPath, [path.join(__dirname, 'fetch-toolchain.js')], {
          stdio: ['ignore', 'pipe', 'inherit'],
          encoding: 'utf8'
        });
        ({ extismPath, binaryenPath: downloadedBinaryenPath } = JSON.parse(output));
      } catch (error) {
        console.error('\n❌ Could not download the WebAssembly toolchain (extism-js and binaryen).');
        console.error('Check your internet connection, or reinstall @vortideck/plugin-sdk from npm, which bundles it.');
        process.exit(1);
      }
    }

    // For WSL, copy extism-js to /tmp with executable permissions
    if (platform === 'linux' && fs.existsSync('/proc/version') &&
        fs.readFileSync('/proc/version', 'utf8').toLowerCase().includes('microsoft')) {
      const tmpExtismPath = path.join('/tmp', 'vortideck-extism-js');
      if (!fs.existsSync(tmpExtismPath)) {
        fs.mkdirSync(tmpExtismPath, { recursive: true });
      }
      const tmpExtismBinary = path.join(tmpExtismPath, 'extism-js');
      fs.copyFileSync(extismPath, tmpExtismBinary);
      fs.chmodSync(tmpExtismBinary, '755');
      extismPath = tmpExtismBinary;
    }

    try {
      // Use bundled binaryen binaries
      let binaryenPath = downloadedBinaryenPath || path.join(__dirname, 'binaryen');

      // Detect which binaryen binaries to use
      let wasmMergeBinary, wasmOptBinary;
      if (platform === 'win32') {
        wasmMergeBinary = 'wasm-merge.exe';
        wasmOptBinary = 'wasm-opt.exe';
      } else if (platform === 'darwin') {
        wasmMergeBinary = arch === 'arm64' ? 'wasm-merge-macos-arm' : 'wasm-merge-macos';
        wasmOptBinary = arch === 'arm64' ? 'wasm-opt-macos-arm' : 'wasm-opt-macos';
      } else if (platform === 'linux') {
        wasmMergeBinary = 'wasm-merge-linux';
        wasmOptBinary = 'wasm-opt-linux';
      }

      // Create copies/symlinks without extensions so extism-js can find them
      const wasmMergePath = path.join(binaryenPath, wasmMergeBinary);
      const wasmOptPath = path.join(binaryenPath, wasmOptBinary);
      const wasmMergeLink = path.join(binaryenPath, 'wasm-merge' + (platform === 'win32' ? '.exe' : ''));
      const wasmOptLink = path.join(binaryenPath, 'wasm-opt' + (platform === 'win32' ? '.exe' : ''));

      // On Unix, ensure bundled binaries are executable and create copies/symlinks
      // (downloaded binaries already have their plain names and permissions)
      if (!downloadedBinaryenPath && platform !== 'win32') {
        // Check if we're on WSL (files might be on Windows filesystem)
        const isWSL = fs.existsSync('/proc/version') &&
                      fs.readFileSync('/proc/version', 'utf8').toLowerCase().includes('microsoft');

        if (isWSL) {
          // WSL: Copy binaries to /tmp with executable permissions
          const tmpBinaryenPath = path.join('/tmp', 'vortideck-binaryen');
          if (!fs.existsSync(tmpBinaryenPath)) {
            fs.mkdirSync(tmpBinaryenPath, { recursive: true });
          }

          const tmpWasmMerge = path.join(tmpBinaryenPath, 'wasm-merge');
          const tmpWasmOpt = path.join(tmpBinaryenPath, 'wasm-opt');

          // Copy binaries to /tmp and make executable
          fs.copyFileSync(wasmMergePath, tmpWasmMerge);
          fs.copyFileSync(wasmOptPath, tmpWasmOpt);
          fs.chmodSync(tmpWasmMerge, '755');
          fs.chmodSync(tmpWasmOpt, '755');

          // Use /tmp path for binaryen
          binaryenPath = tmpBinaryenPath;
        } else {
          // Native Linux/macOS: Ensure source binaries are executable
          try {
            fs.chmodSync(wasmMergePath, '755');
            fs.chmodSync(wasmOptPath, '755');
          } catch (e) {
            // Ignore errors if chmod fails
          }

          if (!fs.existsSync(wasmMergeLink)) {
            fs.copyFileSync(wasmMergePath, wasmMergeLink);
            fs.chmodSync(wasmMergeLink, '755');
          }
          if (!fs.existsSync(wasmOptLink)) {
            fs.copyFileSync(wasmOptPath, wasmOptLink);
            fs.chmodSync(wasmOptLink, '755');
          }
        }
      }

      const env = { ...process.env };
      // Add bundled binaryen directory to PATH
      env.PATH = `${binaryenPath}${path.delimiter}${env.PATH}`;

      // Use bundled extism-js binary with interface file
      execSync(`"${extismPath}" ${bundleFile} -i dist/index.d.ts -o ${outputFile}`, {
        stdio: 'inherit',
        env
      });
      console.log(`\n✅ Plugin WASM compiled successfully: ${outputFile}`);
    } catch (error) {
      console.error('\n❌ WASM compilation failed');
      process.exit(1);
    }

    // Step 4: Generate plugin folder structure
    console.log('\n📁 Generating plugin folder structure...');
    const folderName = `${pluginName}-plugin`;
    const pluginFolder = path.join('dist', folderName);

    // Create plugin folder
    if (!fs.existsSync(pluginFolder)) {
      fs.mkdirSync(pluginFolder, { recursive: true });
    }

    // Step 5: Generate manifest.json from package.json vortideck config
    console.log('📝 Generating manifest.json...');
    const pkg = JSON.parse(fs.readFileSync('package.json', 'utf8'));
    const vortideck = pkg.vortideck || {};
    const packageId = vortideck.id || `${pluginName}.plugin`;
    validatePackageId(packageId);

    const manifest = {
      id: packageId,
      name: vortideck.name || pkg.name,
      version: vortideck.version || pkg.version,
      author: vortideck.author || pkg.author || 'Unknown',
      description: vortideck.description || pkg.description || '',
      vortideck_version: vortideck.vortideck_version || '>=0.7.0',
      schema_version: vortideck.schema_version || '1.0',
      permissions: vortideck.permissions || [],
      web_extensions: vortideck.web_extensions || false,
      main: 'plugin.wasm'
    };

    // Add web config if web_extensions is enabled
    if (manifest.web_extensions && vortideck.web) {
      manifest.web = vortideck.web;
    }

    fs.writeFileSync(
      path.join(pluginFolder, 'manifest.json'),
      JSON.stringify(manifest, null, 2)
    );
    console.log('✅ Generated manifest.json');

    // Step 6: Copy WASM to plugin folder
    fs.copyFileSync(outputFile, path.join(pluginFolder, 'plugin.wasm'));
    console.log('✅ Copied plugin.wasm');

    // Step 7: Copy icon if exists
    const iconExtensions = ['.png', '.jpg', '.jpeg', '.svg'];
    for (const ext of iconExtensions) {
      const iconPath = `icon${ext}`;
      if (fs.existsSync(iconPath)) {
        fs.copyFileSync(iconPath, path.join(pluginFolder, `icon${ext}`));
        console.log(`✅ Copied icon${ext}`);
        break;
      }
    }

    // Step 8: Copy web extensions if they exist
    // Check both src/web/ (new SDK convention) and web/ (old convention)
    const webSrcPath = fs.existsSync('src/web') ? 'src/web' : (fs.existsSync('web') ? 'web' : null);

    if (manifest.web_extensions && webSrcPath) {
      const webFolder = path.join(pluginFolder, 'web');
      fs.mkdirSync(webFolder, { recursive: true });

      // Copy all files from web source directory
      copyDirectoryRecursive(webSrcPath, webFolder);
      console.log(`✅ Copied web extensions from ${webSrcPath}`);
    }

    console.log(`\n🎉 Plugin built successfully!`);
    console.log(`📦 Output folder: ${pluginFolder}/`);
    console.log(`   ├── manifest.json`);
    console.log(`   ├── plugin.wasm`);
    if (fs.existsSync(path.join(pluginFolder, 'icon.png')) ||
        fs.existsSync(path.join(pluginFolder, 'icon.svg'))) {
      console.log(`   ├── icon.*`);
    }
    if (manifest.web_extensions) {
      console.log(`   └── web/`);
    }
    console.log(`\n✨ Ready to install in VortiDeck!`);

  } catch (error) {
    console.error('❌ Build failed:', error.message);
    process.exit(1);
  }
}

/**
 * Recursively copy a directory
 */
function copyDirectoryRecursive(source, target) {
  if (!fs.existsSync(target)) {
    fs.mkdirSync(target, { recursive: true });
  }

  const files = fs.readdirSync(source);
  for (const file of files) {
    const sourcePath = path.join(source, file);
    const targetPath = path.join(target, file);

    if (fs.statSync(sourcePath).isDirectory()) {
      copyDirectoryRecursive(sourcePath, targetPath);
    } else {
      fs.copyFileSync(sourcePath, targetPath);
    }
  }
}

/**
 * Initialize a new plugin project
 */
function initPlugin() {
  console.log('🚀 Creating new VortiDeck plugin...\n');

  // Get plugin name from user or use current directory name
  const pluginName = process.argv[3] || path.basename(process.cwd());
  const pluginId = pluginName.toLowerCase().replace(/[^a-z0-9-]/g, '-');

  // Create directories
  if (!fs.existsSync('src')) {
    fs.mkdirSync('src', { recursive: true });
  }
  if (!fs.existsSync('dist')) {
    fs.mkdirSync('dist', { recursive: true });
  }

  // Create package.json
  const packageJson = {
    name: pluginName,
    version: "1.0.0",
    description: "A VortiDeck plugin",
    main: "dist/index.js",
    scripts: {
      build: "vortideck build",
      dev: "vortideck build --watch"
    },
    keywords: ["vortideck", "plugin", "wasm"],
    author: "",
    license: "MIT",
    vortideck: {
      id: `${pluginId}.example`,
      name: pluginName,
      version: "1.0.0",
      author: "",
      description: "A VortiDeck plugin",
      vortideck_version: ">=0.7.0",
      schema_version: "1.0",
      permissions: [],
      web_extensions: false
    },
    dependencies: {
      "@vortideck/plugin-sdk": "^0.1.0"
    },
    devDependencies: {
      "@extism/js-pdk": "^1.0.1",
      "typescript": "^5.3.2"
    }
  };
  fs.writeFileSync('package.json', JSON.stringify(packageJson, null, 2) + '\n');
  console.log('✅ Created package.json');

  // Create tsconfig.json
  const tsconfig = {
    compilerOptions: {
      target: "ES2020",
      module: "ESNext",
      lib: ["ES2020"],
      strict: true,
      esModuleInterop: true,
      skipLibCheck: true,
      moduleResolution: "bundler",
      outDir: "dist",
      rootDir: "src"
    },
    include: ["src/**/*"],
    exclude: ["node_modules", "dist"]
  };
  fs.writeFileSync('tsconfig.json', JSON.stringify(tsconfig, null, 2) + '\n');
  console.log('✅ Created tsconfig.json');

  // Create .gitignore
  const gitignore = `node_modules/
dist/
*.wasm
*.log
.DS_Store
`;
  fs.writeFileSync('.gitignore', gitignore);
  console.log('✅ Created .gitignore');

  // Create README.md
  const readme = `# ${pluginName}

A VortiDeck plugin built with the VortiDeck Plugin SDK.

## Installation

\`\`\`bash
npm install
\`\`\`

## Development

\`\`\`bash
npm run build
\`\`\`

## Plugin Details

- **Package ID**: \`${pluginId}.example\`
- **Node ID**: \`${pluginId}:example\`
- **Category**: utility
- **Type**: Flow node

## Usage

Edit \`src/main.ts\` to customize your plugin logic.

## Build

\`\`\`bash
npm run build
\`\`\`

Output: \`dist/${pluginName}.wasm\`

## License

MIT
`;
  fs.writeFileSync('README.md', readme);
  console.log('✅ Created README.md');

  // Create example plugin with full functionality
  const examplePlugin = `import { createPlugin, defineNode } from '@vortideck/plugin-sdk'
import vortideck, { FlowParams } from '@vortideck/plugin-sdk'

/**
 * ${pluginName}
 *
 * This is an example VortiDeck plugin that demonstrates:
 * - Using the createPlugin helper with the new multi-node API
 * - Accessing VortiDeck APIs (log, registerNode, etc.)
 * - Processing input parameters
 * - Returning results
 *
 * Note: Plugin metadata (id, name, version, author, permissions) is defined in manifest.json
 * This file only contains the node implementation logic
 */

export const { execute, register, get_metadata } = createPlugin({
  nodes: {
    // Define your plugin nodes here
    // Each node becomes a flow node in VortiDeck
    "example": defineNode({
      metadata: {
        id: "${pluginId}:example",
        name: "${pluginName}",
        description: "An example VortiDeck plugin - customize this!",
        category: "utility",
        type_color: "#4A90E2",
        outputs_count: 1
      },

      execute(input: any, context) {
        // Log to VortiDeck console
        vortideck.log("Plugin ${pluginName} executing...")

        // Parse input parameters
        const params = new FlowParams(input)
        const text = params.getInput("Hello from ${pluginName}!")

        // Process the input (example: convert to uppercase)
        const result = text.toUpperCase()

        // Return success response
        return {
          success: true,
          output: result,
          metadata: {
            processed_at: new Date().toISOString(),
            plugin: "${pluginName}"
          }
        }
      },

      // Optional: Called when this node is registered
      onRegister() {
        vortideck.log("${pluginName} node registered!")
      }
    })
  },

  // Optional: Called when the entire plugin is registered
  onPluginRegister() {
    vortideck.log("${pluginName} plugin initialized!")
  }
})
`;
  fs.writeFileSync('src/main.ts', examplePlugin);
  console.log('✅ Created src/main.ts');

  console.log(`
✨ Plugin initialized successfully!

📦 Plugin: ${pluginName}
🆔 Package ID: ${pluginId}.example
🧩 Node ID: ${pluginId}:example

Plugin configuration is in package.json under the "vortideck" field.
Edit this to customize plugin metadata (name, description, permissions, etc.)

Next steps:
  1. npm install
  2. Edit src/main.ts to customize your plugin logic
  3. Edit package.json "vortideck" field to customize metadata
  4. npm run build
  5. Copy dist/${pluginName}-plugin/ folder to VortiDeck plugins directory

The build process will automatically:
  ✅ Generate manifest.json from package.json
  ✅ Create proper folder structure
  ✅ Copy icon and web extensions (if present)

Happy coding! 🚀
  `);
}
