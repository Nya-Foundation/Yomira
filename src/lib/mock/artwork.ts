// Original vector illustrations for the browser demo. No remote images or account data.
const PALETTES = [
  ["#f4ddba", "#ed997c", "#be6473", "#693f60", "#302e4c"],
  ["#d9ece8", "#9dcac0", "#679f99", "#376e79", "#234458"],
  ["#eee6cd", "#d4c48d", "#9ea478", "#627b68", "#304d4c"],
  ["#dce0ef", "#b5b3d7", "#8e86b3", "#625e8c", "#383855"],
  ["#f5e4c9", "#e8b576", "#cd855d", "#985b51", "#503b47"],
  ["#d6e9f1", "#9cbed4", "#7298bc", "#4e729a", "#2c486a"],
];

const escapeXml = (value: string) => value.replace(/[<>&"']/g, (char) =>
  ({ "<": "&lt;", ">": "&gt;", "&": "&amp;", '"': "&quot;", "'": "&apos;" })[char]!);

function landscape(seed: number): string {
  const [sky, haze, ridge, middle, foreground] = PALETTES[seed % PALETTES.length];
  const sun = 90 + seed % 190;
  const scene = Math.floor(seed / PALETTES.length) % 3;
  const background = `<rect width="360" height="480" fill="${sky}"/>
    <circle cx="${sun}" cy="180" r="43" fill="#fff8e6"/>
    <path d="M-20 238 Q90 192 185 231 T390 211 V480 H-20Z" fill="${haze}"/>`;
  if (scene === 0) return background + `
    <path d="M-30 360 112 186 203 313 274 231 390 369 V480 H-30Z" fill="${ridge}"/>
    <path d="m112 186-33 41 30-11 23 20 13-5Z" fill="#fff8e6" opacity=".8"/>
    <path d="M-20 361 72 294 193 383 307 283 390 327 V480 H-20Z" fill="${middle}"/>
    <path d="M-20 416 Q84 350 181 410 T390 389 V480 H-20Z" fill="${foreground}"/>
    <path d="M135 480 Q263 424 189 397" fill="none" stroke="${sky}" stroke-width="5" opacity=".7"/>`;
  if (scene === 1) return background + `
    <path d="M-20 309 Q79 250 157 293 T390 264 V480 H-20Z" fill="${ridge}"/>
    <path d="M0 330 Q83 304 166 343 T360 323 V480 H0Z" fill="${middle}"/>
    <path d="M195 319 Q108 353 216 386 T151 480 H278 Q352 407 229 380 T216 319Z" fill="${sky}"/>
    ${[18, 52, 94, 288, 331].map((x, i) => `<path d="M${x} ${259 + i * 9} l-30 89 h20 l-31 44 h32 v53 h17 v-53 h30 l-29-44 h20Z" fill="${foreground}"/>`).join("")}`;
  return background + `
    <path d="M0 277 Q71 250 130 281 T360 263 V480 H0Z" fill="${ridge}"/>
    <path d="M0 325 Q140 299 360 329 M0 370 Q170 343 360 373 M0 414 Q150 393 360 416" fill="none" stroke="${sky}" stroke-width="2" opacity=".55"/>
    <path d="M0 370 57 349 88 365 134 480 H0Z M360 346 317 320 293 351 268 480 H360Z" fill="${foreground}"/>
    <path d="m194 310 13-49 14 49Z" fill="#fff8e6"/>
    <path d="M185 315 h47 l-10 10 h-28Z" fill="${middle}"/>`;
}

const uri = (body: string, width: number, height: number) =>
  `data:image/svg+xml;charset=utf-8,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 360 480">${body}</svg>`)}`;

export function demoCover(seed: number, width: number, height: number, title = ""): string {
  const words = title.split(" ");
  return uri(`${landscape(seed)}
    <rect x="16" y="16" width="328" height="448" rx="2" fill="none" stroke="#fff8e6" opacity=".45"/>
    <text x="30" y="42" font-family="sans-serif" font-size="9" letter-spacing="3" fill="#303447">YOMIRA / FIELD NOTES</text>
    <text x="28" y="90" font-family="Georgia,serif" font-size="35" fill="#303447">${escapeXml(words[0] || "Quiet")}</text>
    <text x="28" y="125" font-family="Georgia,serif" font-size="35" fill="#303447">${escapeXml(words.slice(1).join(" ") || "Places")}</text>
    <text x="30" y="448" font-family="sans-serif" font-size="9" letter-spacing="2" fill="#fff8e6">ILLUSTRATED JOURNEYS · ${String(seed % 24 + 1).padStart(2, "0")}</text>`, width, height);
}

export function demoPage(seed: number, index: number, width: number, height: number): string {
  return uri(`<rect width="360" height="480" fill="#f4efe5"/>
    <text x="24" y="30" font-family="sans-serif" font-size="8" letter-spacing="2" fill="#625e58">FIELD NOTES / A QUIET JOURNEY</text>
    <svg x="24" y="48" width="312" height="300" viewBox="0 135 360 345">${landscape(seed + index)}</svg>
    <text x="24" y="378" font-family="Georgia,serif" font-size="23" fill="#303447">Somewhere, slowly.</text>
    <text x="24" y="403" font-family="Georgia,serif" font-size="11" fill="#625e58">A little farther along the trail, the world grows quiet.</text>
    <text x="24" y="421" font-family="Georgia,serif" font-size="11" fill="#625e58">There is time to stop. There is more to discover.</text>
    <path d="M24 447 H336" stroke="#ccc4b8"/>
    <text x="24" y="466" font-family="sans-serif" font-size="8" fill="#625e58">YOMIRA · ORIGINAL DEMO ARTWORK</text>
    <text x="336" y="466" text-anchor="end" font-family="sans-serif" font-size="9" fill="#625e58">${index + 1}</text>`, width, height);
}
