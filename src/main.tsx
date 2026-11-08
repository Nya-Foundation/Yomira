import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "./App";
import { AppNavigationProvider } from "./lib/app-navigation";
import { applyRememberedTheme } from "./lib/settings";
import "./index.css";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      retry: 1,
      staleTime: 30_000,
    },
  },
});

// Before the first paint: the persisted theme only arrives later, over IPC.
applyRememberedTheme();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <AppNavigationProvider><App /></AppNavigationProvider>
      </BrowserRouter>
    </QueryClientProvider>
  </StrictMode>,
);
