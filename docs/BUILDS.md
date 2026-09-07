# Building and signing

Use a clean checkout with the pinned Rust toolchain, Node.js 24.x (24.10+)
and Bun 1.2.18. Install [Tauri's prerequisites](https://v2.tauri.app/start/prerequisites/)
for your build host, then run `bun install --frozen-lockfile`.

The maintained Android project is committed. Do not run `tauri android init`
over it: the manifest, network security resources and `MainActivity.kt` contain
application-specific changes. Tauri regenerates local bindings during builds.

## Windows x64

Build on Windows using Visual Studio C++ build tools, Rust MSVC and WebView2.
When working in WSL, use a separate Windows checkout and Windows toolchains.

```powershell
bun run tauri build --target x86_64-pc-windows-msvc --bundles nsis -- --locked
```

The installer is under
`src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/`.
The standalone `yomira.exe` is in that target's `release/` directory and requires
WebView2. The installer handles the runtime prerequisite. Builds currently
have no Authenticode signing certificate.

## Linux x64

CI builds on Ubuntu 22.04 to avoid requiring a newer glibc baseline. On Ubuntu:

```sh
sudo apt-get install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf libfuse2
bun run tauri build --target x86_64-unknown-linux-gnu --bundles deb,appimage -- --locked
```

Packages are under `src-tauri/target/x86_64-unknown-linux-gnu/release/bundle/`.
AppImage compatibility still depends on the distribution and graphics stack;
verify on the distributions you intend to support.

## macOS

Install Xcode command-line tools. Build each architecture on its matching
runner or host:

```sh
# Apple Silicon; use x86_64-apple-darwin for Intel.
rustup target add aarch64-apple-darwin
bun run tauri build --target aarch64-apple-darwin --bundles dmg -- --locked
```

DMGs are under the target's `release/bundle/dmg/`. Current builds are unsigned
and not notarized; macOS may prevent opening them through its normal security
checks. Developer ID signing and notarization are a future release improvement.

## Android

Install JDK 21, Android SDK platform 36, build-tools 35.0.0 and 36.0.0, and
NDK 28.2.13676358. Set `JAVA_HOME` to your JDK and `ANDROID_HOME` to your SDK.

```sh
export NDK_HOME="$ANDROID_HOME/ndk/28.2.13676358"
rustup target add aarch64-linux-android armv7-linux-androideabi
bun run tauri android build --apk --target aarch64 armv7 --ci -- --locked
```

If reusing a Rust cache from another checkout and Kotlin bindings are missing,
clean the Android artifacts of `tauri` and `wry` before rebuilding:

```sh
cargo clean --manifest-path src-tauri/Cargo.toml --release --target aarch64-linux-android -p tauri -p wry
cargo clean --manifest-path src-tauri/Cargo.toml --release --target armv7-linux-androideabi -p tauri -p wry
```

CI performs this targeted cleanup after restoring its cache.

The universal unsigned APK is
`src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk`.
It supports ARM64 and ARMv7, minimum API 24 (Android 7.0), with target API 36.
An unsigned APK cannot be installed. To sign it, provide these environment
variables from your private secret store:

- `ANDROID_KEYSTORE_BASE64`: base64-encoded PKCS12 or JKS keystore.
- `ANDROID_KEY_ALIAS`: alias in the keystore.
- `ANDROID_KEYSTORE_PASSWORD`: keystore password.
- `ANDROID_KEY_PASSWORD`: key password.

Then run `bash scripts/release/sign-android.sh`. It uses a temporary private
folder, aligns the APK for 16 KB pages, signs it, verifies the signature and
alignment, and places the result in `release-assets/`. It removes the temporary
key on exit. Keep a secure backup of your release key: upgrades require the
same signing identity. A new key will not update installations signed with the
old local test key.

The application ID remains `dev.tauri_exviewer`, and the desktop identifier
remains `dev.tauri-exviewer`, to preserve compatibility with existing installs.
The release manifest permits HTTP for the bundled app origin and loopback image
server; external gallery requests use the Rust HTTPS client. A debug-only
resource permits LAN development.

## Branding and license resources

Every Tauri package build runs `bun run gen:icons` and the frontend build.
The [canonical vector](../design/branding/yomira.svg) supplies desktop and
Android icons. License and attribution files are included as bundle resources.
Commit regenerated icons when changing the master.

## CI and device checks

Pull requests compile all supported targets without signing secrets. Their
Android artifact is explicitly unsigned. Automated releases use the same build
workflow with a synchronized version and Android signing enabled. See
[RELEASING.md](RELEASING.md).

Before announcing a release, check installation, startup, sign-in, pagination,
swipe/keyboard reading, offline reading and data preservation on actual target
devices. Browser regression tests use fixtures and do not exercise the live site.
iOS is not currently initialized or built.
