// @ts-check

import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "astro/config";
import react from "@astrojs/react";
import vercel from "@astrojs/vercel";
import sitemap from "@astrojs/sitemap";

// https://astro.build/config
export default defineConfig({
  site: "https://bridgerouter.com",
  output: "server", // Enable server-side rendering for actions
  adapter: vercel(),
  vite: {
    // @ts-ignore - Type incompatibility between @tailwindcss/vite and Astro's Vite plugin types
    plugins: [tailwindcss()],
  },
  integrations: [
    react(),
    sitemap({
      changefreq: "daily",
      priority: 0.7,
      lastmod: new Date(),
    }),
  ],
  image: {
    remotePatterns: [
      { protocol: "https", hostname: "raw.githubusercontent.com" },
      { protocol: "https", hostname: "cryptologos.cc" },
      { protocol: "https", hostname: "assets.coingecko.com" },
    ],
  },
});
