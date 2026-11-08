import { useEffect, useState, type RefObject } from "react";

/**
 * Content width of an element, tracked through a ResizeObserver.
 *
 * `deps` re-observes when the node itself is replaced — a loading state
 * swapping the scroll container, for instance — which a bare mount effect
 * would miss.
 */
export function useElementWidth(ref: RefObject<HTMLElement | null>, deps: unknown[] = []) {
  const [width, setWidth] = useState(0);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const observer = new ResizeObserver((entries) => {
      setWidth(entries[0]?.contentRect.width ?? 0);
    });
    observer.observe(el);
    return () => observer.disconnect();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
  return width;
}
