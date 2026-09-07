import { describe, expect, test } from "bun:test";
import { parseHosts, formatHosts } from "../src/lib/hosts";
import { nextListPage } from "../src/lib/pagination";

describe("hosts.txt", () => {
  test("accepts IPv4, IPv6, aliases and comments without duplicate addresses", () => {
    const hosts = parseHosts("# header\n104.20.18.168 e-hentai.org repo.e-hentai.org\n::1 localhost # IPv6\n104.20.18.168 E-HENTAI.ORG\n");
    expect(hosts["e-hentai.org"]).toEqual(["104.20.18.168"]);
    expect(hosts.localhost).toEqual(["::1"]);
    expect(parseHosts(formatHosts(hosts))).toEqual(hosts);
  });
  test("rejects malformed input and permits disabling overrides", () => {
    for (const bad of ["999.1.1.1 example.org", "::xyz example.org", "1.2.3.4", "1.2.3.4 https://example.org", "1.2.3.4 __proto__"]) {
      expect(() => parseHosts(bad)).toThrow("Line 1");
    }
    expect(Object.keys(parseHosts(" # empty"))).toHaveLength(0);
  });
});

test("pagination preserves opaque continuations and stops cycles", () => {
  const cursor = "https://e-hentai.org/?next=12345&f_search=example";
  expect(nextListPage(cursor, [0])).toBe(cursor);
  expect(nextListPage(cursor, [0, cursor])).toBeUndefined();
  expect(nextListPage(1, [0])).toBe(1);
  expect(nextListPage(0, [0])).toBeUndefined();
  expect(nextListPage(null, [0])).toBeUndefined();
});
