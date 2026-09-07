import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

const bundleProfile: Plugin = {
  name: "bundle-profile",
  generateBundle(_options, bundle) {
    const chunks = Object.values(bundle).filter((entry) => entry.type === "chunk");
    this.emitFile({
      type: "asset",
      fileName: "bundle-profile.json",
      source: JSON.stringify(chunks.map((chunk) => ({
        file: chunk.fileName,
        modules: Object.entries(chunk.modules).map(([file, data]) => ({
          file,
          bytes: data.renderedLength,
        })),
      })), null, 2),
    });
  },
};

// https://vite.dev/config/
export default defineConfig(({ command, mode }) => ({
  plugins: [react(), tailwindcss(), mode === "profile" && bundleProfile],

  // Expose the session-cookie keys from .env (ipb_member_id, ipb_pass_hash,
  // igneous) to the app so SignIn can pre-fill them; they are only ever used
  // locally and are never sent anywhere except the sign-in command.
  envPrefix: command === "serve" ? ["VITE_", "IPB_", "IGNEOUS"] : ["VITE_"],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. split the big single chunk into stable vendor chunks so page code can
  //    load in parallel and cache independently of the app shell
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          "vendor-react": ["react", "react-dom", "scheduler", "react-router-dom"],
          "vendor-query": ["@tanstack/react-query"],
        },
      },
    },
  },
  // 3. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 4. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
