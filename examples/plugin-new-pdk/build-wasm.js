/**
 * WASM build script - compiles bundled JS to WASM using extism-js
 */

const fs = require('fs');
const { execSync } = require('child_process');

const manifest = JSON.parse(fs.readFileSync('./manifest.json', 'utf8'));
const outputFile = `dist/${manifest.id}.wasm`;

try {
  console.log('🔨 Compiling to WASM with extism-js...');
  execSync(`extism-js dist/index.js -i src/index.d.ts -o ${outputFile}`, {
    stdio: 'inherit'
  });
  console.log(`✅ Generated WASM: ${outputFile}`);
} catch (error) {
  console.error('❌ WASM compilation failed:', error.message);
  process.exit(1);
}
