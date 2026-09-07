/** Best-effort browser persistence shared by the reader and its entry point. */
export function readProgress(gid: number): number {
  try {
    const value = Number(localStorage.getItem(`exviewer.reader.pos.${gid}`));
    return Number.isInteger(value) && value >= 0 ? value : 0;
  } catch {
    return 0;
  }
}

export function saveProgress(gid: number, index: number) {
  try {
    localStorage.setItem(`exviewer.reader.pos.${gid}`, String(index));
  } catch {
    // Reading remains available when storage is full or disabled.
  }
}
