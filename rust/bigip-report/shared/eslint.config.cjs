const tsEslint = require("@typescript-eslint/eslint-plugin");
const tsParser = require("@typescript-eslint/parser");

// Only the new, hand-written search modules are linted/type-strict. The nine
// legacy scripts under ts/ were migrated verbatim from JS (they carry
// `// @ts-nocheck` + `// prettier-ignore` headers) and are cleaned up
// incrementally, not in the restructure commit.
module.exports = [
  {
    ignores: ["dist/**", "node_modules/**"],
  },
  {
    files: ["ts/search/**/*.ts"],
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
