import { useMemo, type ReactNode } from "react";
import DOMPurify from "dompurify";

/**
 * Renders a raw e-hentai comment body. The backend passes the comment's
 * innerHTML verbatim (<br>, <a>, <img> emotes, entities) plus pseudo-BBCode
 * ([url], [img]). The pipeline mirrors EhViewer: convert BBCode first, then
 * sanitize with a strict whitelist, then walk the DOM into React elements.
 * Unsanitized HTML is never injected via dangerouslySetInnerHTML.
 */

function bbcodeToHtml(text: string): string {
  return (
    text
      .replace(/\[url=([^\]]*)\]([\s\S]*?)\[\/url\]/gi, '<a href="$1">$2</a>')
      .replace(/\[url\]([\s\S]*?)\[\/url\]/gi, '<a href="$1">$1</a>')
      .replace(/\[img\]([\s\S]*?)\[\/img\]/gi, '<img src="$1">')
  );
}

function sanitize(text: string): string {
  return DOMPurify.sanitize(bbcodeToHtml(text), {
    ALLOWED_TAGS: ["br", "a", "img"],
    ALLOWED_ATTR: ["href", "src"],
  });
}

/** Split a text node into lines so ">quote" lines can be tinted green. */
function renderText(text: string, keyBase: string): ReactNode[] {
  const lines = text.split("\n");
  const out: ReactNode[] = [];
  lines.forEach((line, i) => {
    if (line.startsWith(">")) {
      out.push(
        <span key={`${keyBase}-g${i}`} className="text-success">
          {line}
        </span>,
      );
    } else {
      out.push(line);
    }
    if (i < lines.length - 1) out.push("\n");
  });
  return out;
}

function renderNode(node: Node, key: string): ReactNode {
  if (node.nodeType === Node.TEXT_NODE) {
    return renderText(node.textContent ?? "", key);
  }
  if (node.nodeType !== Node.ELEMENT_NODE) return null;
  const el = node as Element;
  switch (el.tagName.toLowerCase()) {
    case "br":
      return <br key={key} />;
    case "a":
      return (
        <a
          key={key}
          href={el.getAttribute("href") ?? undefined}
          target="_blank"
          rel="noreferrer noopener"
          className="text-accent hover:underline"
        >
          {Array.from(el.childNodes).map((child, i) => renderNode(child, `${key}-${i}`))}
        </a>
      );
    case "img":
      return (
        <img
          key={key}
          src={el.getAttribute("src") ?? undefined}
          alt=""
          className="inline h-5 w-auto align-[-2px]"
        />
      );
    default:
      return Array.from(el.childNodes).map((child, i) => renderNode(child, `${key}-${i}`));
  }
}

export function CommentBody({ text }: { text: string }) {
  const nodes = useMemo(() => {
    const clean = sanitize(text);
    const doc = new DOMParser().parseFromString(`<div>${clean}</div>`, "text/html");
    return Array.from(doc.body.firstElementChild?.childNodes ?? []).map((node, i) =>
      renderNode(node, `c${i}`),
    );
  }, [text]);
  return <div className="whitespace-pre-wrap break-words text-small leading-6 text-muted-foreground">{nodes}</div>;
}
