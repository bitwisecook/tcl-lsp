const tsEslint = require("@typescript-eslint/eslint-plugin");
const tsParser = require("@typescript-eslint/parser");

// Only the hand-written search modules are linted/type-strict. Eleven of the
// twelve page scripts under src/pages/ carry `// @ts-nocheck` headers and are
// prettier-ignored.
module.exports = [
  {
    ignores: ["dist/**", "public/**", "node_modules/**"],
  },
  {
    files: ["src/search/**/*.ts"],
    languageOptions: {
      parser: tsParser,
      ecmaVersion: "latest",
      sourceType: "module",
    },
    plugins: {
      "@typescript-eslint": tsEslint,
    },
    rules: {
      "no-var": "error",
      "prefer-const": "error",
      "no-unused-vars": "off",
      "@typescript-eslint/no-unused-vars": ["error", { argsIgnorePattern: "^_" }],
    },
  },
];
