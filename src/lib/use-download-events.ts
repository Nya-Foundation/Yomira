import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";
import { DOWNLOAD_CHANGED_EVENT } from "./api/downloads";
import { isTauri } from "./api/transport";

/** One subscription per mounted screen, including cleanup during async setup. */
export function useDownloadEvents(onChange: () => void, enabled = true) {
  const callback = useRef(onChange);
  callback.current = onChange;
  useEffect(() => {
    if (!enabled || !isTauri()) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen(DOWNLOAD_CHANGED_EVENT, () => callback.current()).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    }, () => {
      // Explicit retry/window focus and the queue's active-work polling remain available.
    });
    return () => { disposed = true; unlisten?.(); };
  }, [enabled]);
}
