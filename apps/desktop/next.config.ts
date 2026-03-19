import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  allowedDevOrigins: ["127.0.0.1"],
  output: "export",
  reactStrictMode: true,
  images: {
    unoptimized: true,
  },
};

export default nextConfig;
