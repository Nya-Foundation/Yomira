import { useEffect, useId, useMemo, useState, type ReactNode, type RefObject } from "react";
import { suggestTags, tagDbStatus } from "../../lib/api";
import { NAMESPACE_COLORS } from "../../lib/categories";
import { useDebounced } from "../../lib/use-debounced";
import type { TagSuggestion } from "../../lib/types";
import { useT } from "../../lib/i18n";
import { cn } from "../../lib/utils";

import { tagTokenAt, insertTagSuggestion } from "../../lib/search-tokens";

const MAX_ROWS = 10;
const DEBOUNCE_MS = 150;

export interface TagAutocomplete {
  /** Wire onto the search input. */
  onKeyDown: (e: React.KeyboardEvent<HTMLInputElement>) => void;
  onFocus: () => void;
  onSelect: (e: React.SyntheticEvent<HTMLInputElement>) => void;
  inputProps: React.InputHTMLAttributes<HTMLInputElement>;
  onBlur: () => void;
  /** Dropdown to render inside the input's `relative` container. */
  dropdown: ReactNode;
}

export function useTagAutocomplete(opts: {
  query: string;
  inputRef: RefObject<HTMLInputElement | null>;
  /** Replace the search text with the applied tag already merged in. */
  onApply: (nextText: string) => void;
}): TagAutocomplete {
  const { query, inputRef, onApply } = opts;
  const t = useT();
  const [focused, setFocused] = useState(false);
  const [closed, setClosed] = useState(false);
  const [active, setActive] = useState(0);
  const [suggestions, setSuggestions] = useState<TagSuggestion[]>([]);
  const [unavailable, setUnavailable] = useState(false);
  const [caret, setCaret] = useState(query.length);
  const [loadedToken, setLoadedToken] = useState<string | null>(null);
  const listId = useId();

  const token = useMemo(() => tagTokenAt(query, caret), [query, caret]);
  const tokenQuery = token?.query ?? null;
  const debouncedToken = useDebounced(tokenQuery, DEBOUNCE_MS);

  useEffect(() => {
    if (!focused || !debouncedToken || closed || debouncedToken !== tokenQuery) return;
    let cancelled = false;
    setLoadedToken(null);
    void (async () => {
      try {
        const status = await tagDbStatus();
        const hints = status.available ? await suggestTags(debouncedToken) : [];
        if (cancelled) return;
        setUnavailable(!status.available);
        setSuggestions(hints.slice(0, MAX_ROWS));
        setActive(0);
      } catch {
        if (cancelled) return;
        setSuggestions([]);
        setUnavailable(true);
      }
      if (!cancelled) setLoadedToken(debouncedToken);
    })();
    return () => { cancelled = true; };
  }, [focused, debouncedToken, tokenQuery, closed]);

  useEffect(() => { setClosed(false); }, [query, caret]);

  const open = focused && !closed && tokenQuery !== null
    && loadedToken === tokenQuery && (suggestions.length > 0 || unavailable);

  const apply = (s: TagSuggestion) => {
    if (!token) return;
    const next = insertTagSuggestion(query, token, s);
    onApply(next.text);
    setClosed(true);
    setSuggestions([]);
    setCaret(next.caret);
    requestAnimationFrame(() => {
      inputRef.current?.focus();
      inputRef.current?.setSelectionRange(next.caret, next.caret);
    });
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.nativeEvent.isComposing) return;
    if (!open) {
      if (e.key === "Escape") setClosed(true);
      return;
    }
    if (e.key === "ArrowDown" && suggestions.length > 0) {
      e.preventDefault();
      setActive((a) => (a + 1) % suggestions.length);
    } else if (e.key === "ArrowUp" && suggestions.length > 0) {
      e.preventDefault();
      setActive((a) => (a - 1 + suggestions.length) % suggestions.length);
    } else if ((e.key === "Enter" || e.key === "Tab")) {
      if (suggestions.length === 0) return;
      e.preventDefault();
      apply(suggestions[Math.min(active, suggestions.length - 1)]);
    } else if (e.key === "Escape") {
      e.preventDefault();
      setClosed(true);
    }
  };

  const dropdown = open && (
    <div
      id={listId}
      role="listbox"
      aria-label={t("search.autocomplete.listLabel")}
      className="absolute left-0 right-0 top-full z-50 mt-1.5 overflow-hidden rounded-lg border border-border bg-elevated shadow-xl shadow-black/40"
    >
      {unavailable ? (
        <p className="px-3 py-2.5 text-caption text-faint-foreground">
          {t("search.autocomplete.unavailable")}
        </p>
      ) : (
        suggestions.map((s, i) => {
          const color = NAMESPACE_COLORS[s.namespace] ?? NAMESPACE_COLORS.misc;
          return (
            <button
              type="button"
              id={`${listId}-${i}`}
              key={`${s.namespace}:${s.name}`}
              role="option"
              aria-selected={i === active}
              onMouseEnter={() => setActive(i)}
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => apply(s)}
              className={cn(
                "flex w-full items-center gap-2 px-3 py-2 text-left text-base",
                i === active && "bg-surface-2",
              )}
            >
              <span
                className="shrink-0 rounded px-1 py-px text-[10px] font-semibold uppercase leading-4"
                style={{
                  color,
                  border: `1px solid ${color}55`,
                  backgroundColor: `${color}1f`,
                }}
              >
                {s.namespace}
              </span>
              <span className="truncate text-foreground">
                {s.name ? s.name : `${s.namespace}:`}
              </span>
              {s.translated && (
                <span className="ml-auto shrink-0 truncate pl-3 text-small text-faint-foreground">
                  {s.translated}
                </span>
              )}
            </button>
          );
        })
      )}
    </div>
  );

  return {
    onKeyDown,
    onFocus: () => { setFocused(true); setCaret(inputRef.current?.selectionStart ?? query.length); },
    onBlur: () => setFocused(false),
    onSelect: (e) => setCaret(e.currentTarget.selectionStart ?? query.length),
    inputProps: {
      role: "combobox", "aria-autocomplete": "list", "aria-expanded": open,
      "aria-controls": open ? listId : undefined,
      "aria-activedescendant": open && suggestions.length ? `${listId}-${active}` : undefined,
      autoComplete: "off",
    },
    dropdown,
  };
}
