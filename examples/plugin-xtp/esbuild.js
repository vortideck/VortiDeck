const esbuild = require('esbuild');
const fs = require('fs');
const path = require('path');

// Plugin to embed manifest into WASM and copy manifest.json for external access
const manifestPlugin = {
  name: 'manifest-embedder',
  setup(build) {
    // Embed manifest data into main.ts - use broader filter to catch the file
    build.onLoad({ filter: /main\.ts$/ }, (args) => {
      console.log('🚀 onLoad triggered for:', path.basename(args.path));
      const source = fs.readFileSync(args.path, 'utf8');
      const manifest = JSON.parse(fs.readFileSync('manifest.json', 'utf8'));
      
      // Use exact string matching since the format is consistent
      const originalLine = 'let manifestData: PluginMetadata | null = null;';
      const replacementLine = `// Embedded manifest data from manifest.json
let manifestData: PluginMetadata = ${JSON.stringify(manifest, null, 2)};`;
      
      // Debug: log what we're looking for
      console.log('🔍 Processing main.ts, looking for manifestData...');
      console.log('🎯 Looking for exact line:', `"${originalLine}"`);
      
      // Check if the line exists
      if (!source.includes(originalLine)) {
        console.log('❌ ERROR: Exact manifestData line not found!');
        console.log('📋 Lines containing manifestData:');
        const lines = source.split('\n');
        lines.forEach((line, index) => {
          if (line.includes('manifestData')) {
            console.log(`Line ${index + 1}: "${line}"`);
          }
        });
      } else {
        console.log('✅ Found exact manifestData line in source');
      }
      
      const injectedSource = source.replace(originalLine, replacementLine);
      
      // Debug: check if replacement worked
      if (source === injectedSource) {
        console.log('❌ ERROR: Manifest data replacement failed!');
        console.log('Please check the manifestData declaration in main.ts');
      } else {
        console.log('✅ Successfully embedded manifest data');
        console.log('📦 Embedded plugin:', manifest.name, 'v' + manifest.version);
      }
      
      return { contents: injectedSource, loader: 'ts' };
    });

    // Copy manifest.json to dist folder for external tools (optional)
    build.onEnd(() => {
      const distDir = path.join(__dirname, 'dist');
      const manifestSrc = path.join(__dirname, 'manifest.json');
      const manifest = JSON.parse(fs.readFileSync(manifestSrc, 'utf8'));
      const manifestDst = path.join(distDir, `${manifest.id}.manifest.json`);

      // Ensure dist directory exists
      if (!fs.existsSync(distDir)) {
        fs.mkdirSync(distDir, { recursive: true });
      }

      // Copy manifest.json to dist with plugin ID name (for external discovery)
      fs.copyFileSync(manifestSrc, manifestDst);
      console.log('✓ Embedded manifest into WASM');
      console.log(`✓ Copied manifest.json to dist/${manifest.id}.manifest.json (for external access)`);
    });
  },
};

esbuild
  .build({
    entryPoints: ['src/index.ts'],
    outdir: 'dist',
    bundle: true,
    sourcemap: true,
    minify: false, // might want to use true for production build
    format: 'cjs', // needs to be CJS for now
    target: ['es2020'], // don't go over es2020 because quickjs doesn't support it
    plugins: [manifestPlugin],
  })
