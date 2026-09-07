import { expect, test } from "bun:test";
import { insertTagSuggestion, tagTokenAt } from "../src/lib/search-tokens";

test("suggests every term, preserves exclusions and replaces only the caret's term", () => {
  const query = 'language:english -parody:"tou hou" artist:someone';
  const token = tagTokenAt(query, query.indexOf('hou') + 2)!;
  expect(token.query).toBe('parody:tou_ho');
  const next = insertTagSuggestion(query, token, { namespace: "parody", name: "touhou_project", translated: "" });
  expect(next.text).toBe('language:english -parody:touhou_project artist:someone');
  expect(tagTokenAt('language:english chin', 21)?.query).toBe('chin');
  expect(tagTokenAt('language:english ', 17)).toBeNull();
  expect(tagTokenAt('rating:>=4.5', 12)).toBeNull();
});

test("namespace completion stays inside the term and names containing spaces are quoted", () => {
  const query = 'language:english par';
  const next = insertTagSuggestion(query, tagTokenAt(query, query.length)!, { namespace: "parody", name: "", translated: "" });
  expect(next.text).toBe('language:english parody:');
  expect(next.caret).toBe(next.text.length);
  const named = insertTagSuggestion(next.text, tagTokenAt(next.text, next.caret)!, { namespace: "parody", name: "touhou project", translated: "" });
  expect(named.text).toBe('language:english parody:"touhou project" ');
});
