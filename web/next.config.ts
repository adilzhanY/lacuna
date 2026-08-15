import type { NextConfig } from "next";

// In development the Next dev server proxies /api to the Rust backend, so the
// browser only ever talks to one origin. In release the Rust binary serves the
// exported frontend itself and no proxy is involved.
const backend = process.env.LACUNA_BACKEND ?? "http://127.0.0.1:4000";

const nextConfig: NextConfig = {
  rewrites() {
    return Promise.resolve([
      {
        source: "/api/:path*",
        destination: `${backend}/api/:path*`,
      },
    ]);
  },
};

export default nextConfig;
