import { createContext, useCallback, useContext, useEffect, useLayoutEffect, useState, type ReactNode } from "react";
import { useLocation, useNavigate, useNavigationType } from "react-router-dom";

interface HistoryState { keys: string[]; index: number }
const STORAGE_KEY = "yomira.navigation";

function restoredHistory(key: string): HistoryState {
  try {
    const saved = JSON.parse(sessionStorage.getItem(STORAGE_KEY) ?? "null") as HistoryState | null;
    if (Array.isArray(saved?.keys) && saved.keys.every(k => typeof k === "string")) {
      const index = saved.keys.indexOf(key);
      if (index >= 0) return { keys: saved.keys, index };
    }
  } catch { /* A fresh in-app history is safe when storage is unavailable. */ }
  return { keys: [key], index: 0 };
}

const NavigationContext = createContext<{ canGoBack: boolean; canGoForward: boolean; back: () => boolean; forward: () => void }>({ canGoBack: false, canGoForward: false, back: () => false, forward: () => {} });

export function AppNavigationProvider({ children }: { children: ReactNode }) {
  const location = useLocation();
  const action = useNavigationType();
  const navigate = useNavigate();
  const [history, setHistory] = useState(() => restoredHistory(location.key));

  useLayoutEffect(() => {
    setHistory(previous => {
      if (previous.keys[previous.index] === location.key) return previous;
      if (action === "PUSH") return { keys: [...previous.keys.slice(0, previous.index + 1), location.key], index: previous.index + 1 };
      if (action === "REPLACE") return { keys: previous.keys.map((key, i) => i === previous.index ? location.key : key), index: previous.index };
      const index = previous.keys.indexOf(location.key);
      return index < 0 ? { keys: [location.key], index: 0 } : { ...previous, index };
    });
  }, [location.key, action]);

  useEffect(() => {
    try { sessionStorage.setItem(STORAGE_KEY, JSON.stringify(history)); } catch { /* Best effort. */ }
  }, [history]);

  const fallback = location.pathname.startsWith("/read/")
    ? new URLSearchParams(location.search).get("source") === "download"
      ? "/downloads" : location.pathname.replace(/^\/read\//, "/gallery/")
    : location.pathname !== "/" ? "/" : null;
  const canGoBack = history.index > 0 || fallback !== null;
  const canGoForward = history.index < history.keys.length - 1;

  const back = useCallback(() => {
    // Android Back dismisses the top sheet/dialog before changing routes.
    const dialog = document.activeElement?.closest("dialog[open]")
      ?? Array.from(document.querySelectorAll("dialog[open]")).slice(-1)[0];
    if (dialog) { dialog.dispatchEvent(new Event("cancel", { cancelable: true })); return true; }
    if (history.index > 0) { void navigate(-1); return true; }
    if (fallback) { void navigate(fallback, { replace: true }); return true; }
    return false;
  }, [history.index, fallback, navigate]);
  const forward = useCallback(() => {
    if (canGoForward && !document.querySelector("dialog[open]")) void navigate(1);
  }, [canGoForward, navigate]);

  useEffect(() => {
    const onBack = (event: Event) => { if (back()) event.preventDefault(); };
    const onKey = (event: KeyboardEvent) => {
      if (!event.altKey || event.ctrlKey || event.metaKey || event.isComposing) return;
      if (event.key === "ArrowLeft") { event.preventDefault(); back(); }
      if (event.key === "ArrowRight") { event.preventDefault(); forward(); }
    };
    window.addEventListener("yomira:back", onBack);
    window.addEventListener("keydown", onKey);
    return () => { window.removeEventListener("yomira:back", onBack); window.removeEventListener("keydown", onKey); };
  }, [back, forward]);

  return <NavigationContext.Provider value={{ canGoBack, canGoForward, back, forward }}>{children}</NavigationContext.Provider>;
}

export const useAppNavigation = () => useContext(NavigationContext);
