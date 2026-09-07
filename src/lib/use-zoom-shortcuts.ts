import { useEffect } from "react";
import { useSettings } from "./settings";

export type ZoomAction = "in" | "out" | "reset";

export function steppedZoom(current: number, action: ZoomAction, min = 0.75, max = 2) {
  if (action === "reset") return 1;
  return Math.max(min, Math.min(max, Math.round((current + (action === "in" ? 0.1 : -0.1)) * 100) / 100));
}

/** Reader components consume these gestures to zoom the image; elsewhere the
 * same shortcuts change the persisted UI scale without browser/WebView zoom. */
export function useReaderZoom(onZoom: (action: ZoomAction) => void) {
  useEffect(() => {
    const listener = (event: Event) => {
      if (document.querySelector("dialog[open]")) return;
      event.preventDefault();
      onZoom((event as CustomEvent<ZoomAction>).detail);
    };
    window.addEventListener("yomira:zoom", listener);
    return () => window.removeEventListener("yomira:zoom", listener);
  }, [onZoom]);
}

export function useZoomShortcuts(enabled: boolean) {
  useEffect(() => {
    if (!enabled) return;
    let lastWheel = -Infinity;
    const zoom = (action: ZoomAction) => {
      if (!window.dispatchEvent(new CustomEvent("yomira:zoom", { detail: action, cancelable: true }))) return;
      const state = useSettings.getState();
      state.update({ displayScale: steppedZoom(state.settings.displayScale, action) });
    };
    const keydown = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey) || event.altKey || event.isComposing) return;
      const action = ["+", "="].includes(event.key) ? "in" : event.key === "-" ? "out" : event.key === "0" ? "reset" : null;
      if (!action) return;
      event.preventDefault();
      zoom(action);
    };
    const wheel = (event: WheelEvent) => {
      if (!(event.ctrlKey || event.metaKey) || !event.deltaY) return;
      event.preventDefault();
      event.stopPropagation();
      const now = performance.now();
      if (now - lastWheel < 80) return;
      lastWheel = now;
      zoom(event.deltaY < 0 ? "in" : "out");
    };
    window.addEventListener("keydown", keydown);
    window.addEventListener("wheel", wheel, { passive: false, capture: true });
    return () => {
      window.removeEventListener("keydown", keydown);
      window.removeEventListener("wheel", wheel, true);
    };
  }, [enabled]);
}
