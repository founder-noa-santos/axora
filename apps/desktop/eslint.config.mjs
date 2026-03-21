import nextCoreVitals from "eslint-config-next/core-web-vitals";
import nextTypescript from "eslint-config-next/typescript";

const config = [
  {
    ignores: ["dist/**", "node_modules/**", ".next/**", "out/**", "release/**"],
  },
  ...nextCoreVitals,
  ...nextTypescript,
  {
    files: ["components/ai-elements/**/*.{ts,tsx}"],
    rules: {
      // Registry-generated; keep files untouched — relax rules that conflict with upstream.
      "react-hooks/static-components": "off",
      "react-hooks/refs": "off",
      "@next/next/no-img-element": "off",
      "jsx-a11y/role-has-required-aria-props": "off",
      "@typescript-eslint/no-unused-vars": "off",
    },
  },
];

export default config;
