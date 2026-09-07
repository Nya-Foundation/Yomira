# Yomira — Folded Y

Two facing pages form the initial Y, with a central opening and short spine.
The mark uses flat fills and simple curves that remain legible at icon sizes.

## Source and regeneration

`yomira.svg` is the editable master. Its charcoal tile is `#0a0a0c` and the
mark is rose `#fb7185`. Keep the central opening clear at small sizes.

Run `bun run gen:icons` from the project root after changing the master.
Release builds run this automatically before compiling the frontend.
The script uses the installed Tauri CLI and generates:

- Desktop PNG, ICO and ICNS assets in `src-tauri/icons`.
- Android legacy, adaptive and monochrome launcher resources. Tauri updates
  an initialized Android project directly; otherwise it exports mobile icons.
- Ignored iOS icon exports for future packaging.
- `public/yomira.svg` for the sidebar and favicon, `public/yomira.png` as the
  PNG fallback, and `public/yomira-mark.svg` as the transparent symbol.

The Android foreground is scaled to leave room for launcher masks and motion;
its background is an independent solid charcoal layer. Monochrome launchers
can recolor the white silhouette. Do not bake a rounded tile into that layer.
The manifest declares both square and round launcher resources; both use the
same adaptive layers on Android 8+. Windows NSIS installer and uninstaller
icons are explicitly configured separately from the application's ICO file.

The production vector and platform exports are generated locally from geometry.
Rebuild application packages after updating the icons.
