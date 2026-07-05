const tsEslint = require("@typescript-eslint/eslint-plugin");
const tsParser = require("@typescript-eslint/parser");

// Only the new, hand-written search modules are linted/type-strict. The nine
// legacy page scripts under src/pages/ were migrated verbatim from JS (they
// carry `// @ts-nocheck` headers + are prettier-ignored) and are cleaned up
// incrementally, not in the restructure commit.
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
