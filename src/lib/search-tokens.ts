import type { TagSuggestion } from "./types";

export interface SearchToken {
  start: number;
  end: number;
  prefix: string;
  query: string;
}

/** Whitespace splits terms only outside quotes. Replace the term at the caret,
 * including its suffix, without touching earlier or later search terms. */
export function tagTokenAt(text: string, caret: number): SearchToken | null {
  caret = Math.max(0, Math.min(caret, text.length));
  const terms = text.matchAll(/(?:[^\s"\\]|\\.|"(?:[^"\\]|\\.)*(?:"|$))+/g);
  for (const term of terms) {
    const start = term.index;
    const end = start + term[0].length;
    if (caret <= start || caret > end) continue;
    const partial = text.slice(start, caret);
    const prefix = partial.match(/^[-~]+/)?.[0] ?? "";
    const query = partial.slice(prefix.length).replace(/"/g, '').replace(/\$$/, '').replace(/ /g, '_');
    if (!query || /^(?:rating|pages|uploader|uploaduid|gid|title|comment|favnote):/i.test(query)) return null;
    return { start, end, prefix, query };
  }
  return null;
}

export function insertTagSuggestion(text: string, token: SearchToken, suggestion: TagSuggestion) {
  // The packed tag DB uses underscores; both quoted names and that format
  // work with EH's search syntax. Quote actual spaces supplied by a source.
  const name = /\s/.test(suggestion.name) ? `"${suggestion.name.replace(/"/g, '\\"')}"` : suggestion.name;
  const tag = `${token.prefix}${suggestion.namespace}:${name}`;
  const suffix = text.slice(token.end);
  const separator = suggestion.name && !/^\s/.test(suffix) ? " " : "";
  const inserted = tag + separator;
  return { text: text.slice(0, token.start) + inserted + suffix, caret: token.start + inserted.length };
}
