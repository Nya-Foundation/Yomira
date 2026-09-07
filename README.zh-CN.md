<p align="center">
  <img src="public/yomira.svg" width="88" height="88" alt="Yomira 标志" />
</p>
<h1 align="center">Yomira</h1>
<p align="center">用心打造的浏览与阅读空间。</p>
<p align="center">
  <a href="README.md">English</a> · <strong>简体中文</strong> · <a href="README.ja.md">日本語</a>
</p>
<p align="center">
  <a href="https://github.com/Nya-Foundation/Yomira/actions/workflows/ci.yml">持续集成</a> ·
  <a href="https://github.com/Nya-Foundation/Yomira/releases">下载</a> ·
  <a href="docs/ROADMAP.md">路线图</a> ·
  <a href="CONTRIBUTING.md">参与贡献</a>
</p>

Yomira 是面向 Android 和桌面平台的非官方 E-Hentai / ExHentai 客户端，
使用 Rust、Tauri 2 和 React 构建。项目受到
[Ehviewer_CN_SXJ](https://github.com/xiaojieonly/Ehviewer_CN_SXJ) 的启发，
将熟悉的画廊浏览与阅读体验融入简约界面，适配触控、键盘和鼠标操作。

本项目正在积极开发中，与 E-Hentai 或 EhViewer 维护者无隶属关系。
所访问的服务包含成人内容，仅限已达到所在司法辖区法定成年年龄的用户使用。

## 浏览、阅读与保存

- **发现想读的内容。** 支持首页、热门、关注和排行榜浏览；提供缩略图和详细列表布局；
  可按分类、评分和页数筛选，支持以图搜图、多标签自动补全和保存搜索关键词。
- **查看画廊详情。** 标签及中文翻译、画廊预览、评分、评论、在线收藏与收藏备注。
- **舒适阅读。** 分页和连续阅读模式、滑动翻页、从右向左阅读、缩放与平移，
  支持调整预加载数量并保存阅读位置。
- **离线保存画廊。** 持久化下载队列，支持暂停、继续、重试和标签管理。
  无需重新获取画廊信息即可阅读已保存的页面，也支持尚未下载完成的画廊。
- **按喜好调整。** 浅色与深色主题、中英文界面、界面缩放，
  支持桌面端前进、后退和缩放快捷操作，以及 Android 返回导航。
- **管理网络连接。** 支持代理、DNS over HTTPS、内置和自定义 hosts、
  图片分辨率偏好设置，以及有容量上限的磁盘图片缓存。

Yomira 尚未涵盖 EhViewer 的全部功能。本地收藏、MyTags 编辑、备份与导入、
Android 原生后台下载服务和 iOS 支持仍在[路线图](docs/ROADMAP.md)中。
性能测量结果及其局限性见[性能文档](docs/PERFORMANCE.md)。

## 界面预览

以下截图使用 Non-H 分类的虚构演示画廊，配有适合公开展示的原创插画，
不包含账户数据或来自服务端的图片。

![桌面端画廊浏览](docs/screenshots/gallery-list.png)

<p align="center">
  <img src="docs/screenshots/gallery-list-mobile.png" width="280" alt="窄屏画廊浏览界面" />
  <img src="docs/screenshots/settings-light-mobile.png" width="280" alt="浅色主题设置界面" />
</p>

## 获取 Yomira

从 [GitHub Releases](https://github.com/Nya-Foundation/Yomira/releases) 下载安装包。

| 平台 | 安装包 | 构建情况 |
| --- | --- | --- |
| Android 7.0+ | 通用 APK：ARM64 和 ARMv7 | CI 构建；发布版本已签名 |
| Windows x64 | `.exe` 安装程序 | CI 构建；未签名 |
| Linux x64 | `.deb`、`.AppImage` | 以 Ubuntu 22.04 为构建基线 |
| macOS | 适用于 Apple Silicon 和 Intel 的 `.dmg` | CI 构建；未签名且未经公证 |

构建通过并不能替代真机测试。依赖要求及平台限制见[构建与签名](docs/BUILDS.md)。

## 本地开发

安装 [Tauri 各平台所需的依赖](https://v2.tauri.app/start/prerequisites/)、
Rust（仓库固定为 1.92.0）、Node.js 24.x（24.10 或更高版本）和 Bun 1.2.18。

```sh
git clone https://github.com/Nya-Foundation/Yomira.git
cd Yomira
bun install --frozen-lockfile
bun run tauri dev
```

开发前端时，运行 `bun run dev` 可在浏览器中打开使用虚构数据的演示界面。
原生网络与文件系统功能需要在 Tauri 应用中运行。

```sh
bun run build                 # TypeScript 检查与前端生产构建
bun run test:unit             # 前端单元测试
bunx playwright install chromium
bun run test:e2e              # 浏览器回归测试
cargo test --locked --manifest-path src-tauri/Cargo.toml
```

桌面端打包：`bun run tauri build`。Android 构建说明和自动化发布流程见
[BUILDS.md](docs/BUILDS.md) 和 [RELEASING.md](docs/RELEASING.md)。

## 项目结构

```text
src/                    React 界面、路由、类型化 IPC 封装与状态管理
src-tauri/src/          Rust 网络请求、解析器、持久化与下载
src-tauri/gen/android/  持续维护的 Android 工程与原生集成
design/branding/        标准标志与图标生成说明
tests/ + e2e/           单元测试、发布工具测试与浏览器回归测试
scripts/               品牌资源、性能分析与发布工具
```

界面通过 `src/lib/api/` 中按业务领域组织的封装调用 Rust。
浏览器演示数据仅在 Tauri 之外加载。Rust 负责网络访问、SQLite 持久化、下载和图片缓存。
详见[架构与维护](docs/MAINTENANCE.md)、[设计系统](docs/DESIGN_SYSTEM.md)
和[离线阅读](docs/OFFLINE_READING.md)。

应用数据保存在各平台的应用目录中。会话 Cookie 保存在本地的 `cookies.json` 中；
下载的画廊和缓存与源码目录分开存放。为保留已有安装的数据，项目有意沿用旧的应用标识符
和 `exviewer.db`。提交问题报告时，请勿包含 Cookie 或个人画廊。

## 致谢与许可证

Yomira 基于 **xiaojieonly 与 Ehviewer_CN_SXJ 贡献者**，以及
**Hippo Seven 与原版 EhViewer 贡献者**的工作构建。
画廊解析器、URL 构造、客户端行为，以及部分下载和标签处理逻辑，均由这些项目的代码改编并移植到 Rust。
中文标签查询使用 [Mapaler/EhViewer](https://github.com/Mapaler/EhViewer) 维护的翻译数据发行版。

本应用以 **GPL-3.0-only** 许可证分发。上游采用 Apache-2.0 的部分保留其原有条款和署名。
请参阅 [LICENSE](LICENSE)、[NOTICE](NOTICE) 和[第三方声明](THIRD_PARTY_NOTICES.md)，
其中包含本次移植所依据的具体上游修订版本的许可详情。

感谢 Rust、Tauri、React 以及更广泛的开源社区；这些社区提供的库让本项目成为可能。

本文链接的其他项目文档目前主要以英文提供。
