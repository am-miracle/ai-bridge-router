// @ts-check

import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "astro/config";
import react from "@astrojs/react";
import node from "@astrojs/node";

// https://astro.build/config
export default defineConfig({
    output: "server", // Enable server-side rendering for actions
    adapter: node({
        mode: "standalone",
    }),

    vite: {
        // @ts-ignore - Type incompatibility between @tailwindcss/vite and Astro's Vite plugin types
        plugins: [tailwindcss()],
    },

    integrations: [react()],
});
