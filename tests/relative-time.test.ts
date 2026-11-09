import { expect, test } from "bun:test";
import { useSettings } from "../src/lib/settings";
import { formatRelativeTime } from "../src/lib/utils";

const ago = (seconds: number) => Math.floor(Date.now() / 1000) - seconds;
const withLanguage = <T,>(language: "en" | "zh-CN", run: () => T): T => {
  const previous = useSettings.getState().settings;
  useSettings.setState({ settings: { ...previous, language } });
  try {
    return run();
  } finally {
    useSettings.setState({ settings: previous });
  }
};

test("relative times follow the chosen language", () => {
  withLanguage("en", () => {
    expect(formatRelativeTime(ago(10))).toBe("just now");
    expect(formatRelativeTime(ago(5 * 60))).toBe("5 min ago");
    expect(formatRelativeTime(ago(3 * 3600))).toBe("3 h ago");
    expect(formatRelativeTime(ago(2 * 86400))).toBe("2 d ago");
  });
  withLanguage("zh-CN", () => {
    expect(formatRelativeTime(ago(10))).toBe("刚刚");
    expect(formatRelativeTime(ago(5 * 60))).toBe("5 分钟前");
    expect(formatRelativeTime(ago(3 * 3600))).toBe("3 小时前");
    expect(formatRelativeTime(ago(2 * 86400))).toBe("2 天前");
  });
});

test("dates older than a month use the chosen language, not the webview's", () => {
  const old = ago(200 * 86400);
  const en = withLanguage("en", () => formatRelativeTime(old));
  const zh = withLanguage("zh-CN", () => formatRelativeTime(old));
  expect(en).toBe(new Date(old * 1000).toLocaleDateString("en"));
  expect(zh).toBe(new Date(old * 1000).toLocaleDateString("zh-CN"));
  expect(en).not.toBe(zh);
});
