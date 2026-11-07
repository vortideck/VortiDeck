import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const typescriptPackage = require("typescript/package.json");
const expectedEngine = "tsgo.7.0.2";

if (
  typescriptPackage.name !== "typescript-native-bridge" ||
  !typescriptPackage.version.endsWith(expectedEngine)
) {
  console.error(
    `Expected the Vue-compatible TypeScript 7 bridge (${expectedEngine}), but resolved ` +
      `${typescriptPackage.name}@${typescriptPackage.version}.`,
  );
  process.exit(1);
}

console.log(
  `TypeScript engine verified: ${expectedEngine} via ` +
    `${typescriptPackage.name}@${typescriptPackage.version}.`,
);
