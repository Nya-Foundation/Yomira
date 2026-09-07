# Yomira interface language

A calm, content-first gallery and reading app. Preserve the zinc/rose identity, keep common actions visible, and reveal advanced controls when needed. This is an app interface, so marketing-page conventions such as hero composition, decorative imagery and scroll choreography do not apply.

Design read: minimalist daily-use media app. Design variance 4/10, motion 2/10, density 5/10 by default with a compact setting. The foundation remains the existing React, Tailwind and Lucide components. No new UI or animation dependency was added.

## Tokens and hierarchy

| Role | Dark | Light |
| --- | --- | --- |
| Canvas | `#0a0a0c` | `#fafafa` |
| Surface | `#141417` | `#ffffff` |
| Secondary surface | `#1b1b20` | `#f4f4f5` |
| Main text | `#f4f4f5` | `#18181b` |
| Supporting text | `#a1a1aa` | `#52525b` |
| Faint text | `#9696a0` | `#666670` |
| Accent | `#fb7185` | `#e11d48` |

Use semantic colors from `src/index.css`, not page-specific gray values. Rose indicates selection and primary action; success/warning/error colors retain their status meanings. Category colors may communicate category identity, but should not compete with every cover.

System sans-serif avoids a network font request and works across Latin/CJK content. Use the existing rem-based title, emphasis, body, small and caption tokens. Monospace is reserved for technical values; tabular numerals are appropriate for page/progress counts. Avoid hard-coded pixel font sizes in scalable content.

The shared `cn()` utility registers these custom type names with tailwind-merge. Keep that registration in sync with the theme; otherwise a font-size class can be mistaken for a text-color override.

Controls use an 8px radius, covers/panels 10px, tag/filter chips full rounding. Default spacing follows 4/8/12/16/20/24px increments. Prefer space and a restrained separator over nested bordered cards. Cover images keep a predictable 3:4 slot in browsing; visible metadata lives beneath the image.

## Layout

- **1280px and above:** labeled sidebar, grouped by spacing, with account/settings at the bottom.
- **768–1279px:** compact rail with accessible names and native hover titles.
- **Below 768px:** fixed Home, Favorites, Downloads and More. More contains every secondary destination and account/theme access. Do not make the bottom navigation horizontally scroll.
- The shell reserves bottom navigation and device safe-area space once. Pages should not add their own duplicate mobile bottom-bar padding.
- Main content owns a bounded scroll area. List pages give the grid the remaining height. Settings, History, News, detail and sign-in own their own scroll containers.
- Discovery search is full-width on small screens. Secondary tools wrap; icon-only actions retain accessible names. Detail uses a smaller cover so reading actions appear earlier.
- Gallery columns respond to available width and display scale; actual rendered rows are measured. Do not reintroduce a fixed text-height estimate as the final row size.
- Empty, loading and error states retain their page's usable scroll and retry behavior.

## Interaction

- Use the shared Button/Input/Select/Sheet/Dialog components. Native modal dialogs make background content inert; the shared Modal restores focus and wraps keyboard navigation inside the panel.
- Modal open/close effects depend on visibility, not changing callback identity. Editing a value must not reinitialize focus.
- Escape dismisses the current modal. Global shortcuts must not operate the underlying page while a modal is open.
- Tabs support arrow keys and Home/End, with the active tab in the normal tab order.
- Reader controls stay visible until explicitly hidden. A visible, named reveal button remains available. Keyboard/touch users must not depend on mouse movement to recover controls.
- The reader's back action returns to gallery detail for online reading and Downloads for offline reading. Page zero is an explicit request, distinct from an absent page parameter.
- Destructive operations retain existing confirmation flows. Content filters and navigation do not need confirmation.
- Touch controls have at least a 44px height where the shared touch rule applies; secondary delete controls become visible on coarse pointers. This is not a complete target-size audit.
- Subtle 150–180ms state transitions only. Honor `prefers-reduced-motion`; do not animate every cover during hover or scrolling.

## Review captures

These captures use the existing synthetic demo fixtures, not authenticated gallery content.

| Desktop | Narrow screens |
| --- | --- |
| [Gallery list](screenshots/gallery-list.png) | [Gallery list](screenshots/gallery-list-mobile.png) |
| [Gallery detail](screenshots/gallery-detail.png) | [More navigation](screenshots/navigation-mobile.png) |
| [Reader](screenshots/reader.png) | [Downloads](screenshots/downloads-mobile.png) |
| | [Light settings](screenshots/settings-light-mobile.png) |

## Remaining design work

Add status/search/sort tools to Downloads when their semantics are implemented; use named favorites collections; expose complete saved searches; add a compact metadata list. These are interaction changes and should ship with their data model, not as decorative controls.

The present light/dark tokens improve faint-text contrast, but this pass is not a full WCAG audit. Verify all tag/category combinations, custom controls, announcement behavior, focus order, large CJK text, browser zoom, narrow landscape windows and real touch devices before claiming accessibility compliance. Verify native dialog behavior in each shipped Tauri webview.
