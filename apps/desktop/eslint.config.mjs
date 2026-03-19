import nextCoreVitals from "eslint-config-next/core-web-vitals";
import nextTypescript from "eslint-config-next/typescript";

const config = [
  {
    ignores: ["dist/**", "node_modules/**", ".next/**", "out/**", "release/**"],
  },
  ...nextCoreVitals,
  ...nextTypescript,
];

export default config;
