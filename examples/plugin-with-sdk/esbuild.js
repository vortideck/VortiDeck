const esbuild = require('esbuild');
const path = require('path');

esbuild
  .build({
    entryPoints: ['src/main.ts'],
    outdir: 'dist',
    bundle: true,
    sourcemap: true,
    minify: false,
    format: 'cjs',
    target: ['es2020'],
    external: ['@extism/js-pdk'], // Extism PDK is provided by runtime
  })
  .then(() => console.log('✅ Bundle created successfully'))
  .catch(() => process.exit(1));
