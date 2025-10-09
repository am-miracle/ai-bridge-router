// @ts-check

import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "astro/config";
import react from "@astrojs/react";
import vercel from "@astrojs/vercel";
import sitemap from "@astrojs/sitemap";

// https://astro.build/config
export default defineConfig({
  site: "https://bridgerouter.com",
  output: "server",
  adapter: vercel({
    webAnalytics: { enabled: true },
    imageService: true,
    imagesConfig: {
      sizes: [320, 640, 1280, 1920],
      formats: ["image/webp", "image/avif"],
    },
  }),
  vite: {
    // @ts-ignore - Type incompatibility between @tailwindcss/vite and Astro's Vite plugin types
    plugins: [tailwindcss()],
    build: {
      cssCodeSplit: true,
      rollupOptions: {
        output: {
          manualChunks: {
            "react-vendor": ["react", "react-dom"],
            motion: ["framer-motion"],
            radix: ["@radix-ui/react-dialog", "@radix-ui/react-select", "@radix-ui/react-label"],
            ui: ["lucide-react", "class-variance-authority", "clsx", "tailwind-merge"],
          },
        },
      },
    },
  },
  integrations: [
    react({
      experimentalReactChildren: false,
    }),
    sitemap({
      changefreq: "daily",
      priority: 0.7,
      lastmod: new Date(),
    }),
  ],
  image: {
    service: {
      entrypoint: "astro/assets/services/sharp",
    },
    remotePatterns: [
      { protocol: "https", hostname: "raw.githubusercontent.com" },
      { protocol: "https", hostname: "assets.coingecko.com" },
    ],
  },
  prefetch: {
    prefetchAll: true,
    defaultStrategy: "viewport",
  },
  compressHTML: true,
  build: {
    inlineStylesheets: "auto",
  },
});
