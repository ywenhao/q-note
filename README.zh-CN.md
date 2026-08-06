<p align="center">
  <img src="./app-icon.png" alt="Q Note 图标" width="120" height="120" />
</p>

<h1 align="center">Q Note</h1>

<p align="center">
  一个小而快的桌面便签面板，用来保存经常复制的文本、图片、链接和本地文件路径。
</p>

<p align="center">
  <a href="./README.md">English</a>
</p>

## 截图

| 主面板                                                                      | 编辑窗口                                                                        |
| --------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| <img src="./docs/images/q-note-main.png" alt="Q Note 主面板" width="288" /> | <img src="./docs/images/q-note-editor.png" alt="Q Note 编辑窗口" width="360" /> |

## 项目定位

Q Note 基于 Tauri 2、Vue 3.6 Vapor、TypeScript、Vite+、Tailwind CSS、SQLite、Drizzle 和 vue-draggable-plus 构建。应用面向轻量桌面使用场景：打开快、界面紧凑、卡片列表可扫描，适合保存代码片段、常用回复、图片素材、本地路径和临时截图。

## 核心功能

| 功能       | 说明                                                                              |
| ---------- | --------------------------------------------------------------------------------- |
| 中英文切换 | 顶部语言按钮可切换中文和英文，并保存到本地                                        |
| 快速复制   | 点击卡片默认复制文本；纯附件便签会复制附件值                                      |
| 卡片置顶   | 每张卡片可单独置顶，置顶卡片优先展示                                              |
| 拖拽排序   | 卡片可拖拽排序，拖过置顶/未置顶分界时会自动切换置顶状态                           |
| 颜色标记   | 提供 12 种预设卡片背景色，并与主背景色 `#ffd150` 保持协调                         |
| 卡片高度   | 卡片默认最多两行，底部手柄拖拽后按完整行高吸附                                    |
| 图片预览   | 编辑器里的图片缩略图支持点击查看大图                                              |
| 文件拖拽   | 本地拖入的文件保存真实路径；网页图片优先保存 URL；无路径/URL 的文件用 base64 兜底 |
| 全部删除   | 工具栏和右键菜单支持全部删除，删除前会弹出红色确认按钮                            |
| 窗口置顶   | 顶部图标或右键菜单可切换窗口最高层级                                              |
| 状态栏图标 | 系统托盘常驻黄色 Q 图标，点击可唤起主窗口                                         |
| 开机自启   | 设置里可开启或关闭开机自启动，默认关闭                                            |
| Q 图标模式 | 点击折叠图标后变成黄色 Q 图标，可拖动、贴边吸附、悬停展开                         |
| 数据持久化 | 便签、附件、颜色、卡片顺序、卡片高度、窗口大小、置顶状态和语言保存到 SQLite       |
| 导入导出   | 便签和本地配置可导出为 JSON，也可从 JSON 导入恢复                                 |

## 编辑器附件规则

| 来源                 | 保存方式                     |
| -------------------- | ---------------------------- |
| 桌面端拖入本地图片   | 保存本地路径，显示图片缩略图 |
| 桌面端拖入普通文件   | 保存本地路径，显示文件条     |
| 浏览器拖入网页图片   | 优先保存图片 URL             |
| 浏览器拖入文件对象   | 读取为 base64 data URL       |
| 粘贴截图或剪贴板图片 | 读取为 base64 data URL       |
| 手动输入 URL 或路径  | 根据扩展名判断图片或文件     |

## 开发命令

```bash
pnpm install
pnpm dev
pnpm tauri dev
pnpm typecheck
pnpm check
pnpm check:fix
pnpm format
pnpm format:check
pnpm build
```

## 技术栈

| 层级     | 技术                                                      |
| -------- | --------------------------------------------------------- |
| 桌面容器 | Tauri 2                                                   |
| 前端     | Vue 3.6 Vapor + TypeScript + `<script setup>`             |
| 构建     | Vite 8 + Vite+                                            |
| 样式     | Tailwind CSS 4 + CSS                                      |
| 拖拽排序 | vue-draggable-plus + SortableJS                           |
| 数据     | SQLite + `@tauri-apps/plugin-sql` + Drizzle proxy         |
| 文件     | `@tauri-apps/plugin-dialog` + `@tauri-apps/plugin-fs`     |
| 图标     | 由原生 Vapor 组件渲染的 Lucide 图形数据 + 黄色 Q 应用图标 |

所有 Vue SFC 都显式启用 Vapor 编译。应用状态使用 `ref`、`computed`、`watch`、Vue 生命周期和 composables 管理，不包含 React 运行时，也没有引入 VDOM 兼容层。实现细节见 [Vue Vapor 架构说明](./docs/vue-vapor-migration.md)。

## 数据说明

应用使用 SQLite 保存数据，数据库位置为：

| 平台        | 路径                                  |
| ----------- | ------------------------------------- |
| Windows     | `C:\Users\<用户名>\.q-note\q-note.db` |
| macOS/Linux | `~/.q-note/q-note.db`                 |

如果旧版本 Windows 数据仍在 `%APPDATA%\com.win11.q-note\q-note.db`，应用首次启动时会自动复制到新位置，不会删除旧文件。

导出的 JSON 包含：

| 字段         | 内容                                                       |
| ------------ | ---------------------------------------------------------- |
| `notes`      | 便签文本、颜色、置顶状态、卡片顺序、卡片高度和附件         |
| `settings`   | 语言、窗口置顶、Q 图标位置、窗口位置、窗口大小和开机自启动 |
| `exportedAt` | 导出时间                                                   |
| `version`    | 导出格式版本                                               |

## 发布

维护者可以用一条命令发版：

```bash
pnpm release:patch   # 0.2.5 -> 0.2.6
pnpm release:minor   # 0.2.5 -> 0.3.0
pnpm release:major   # 0.2.5 -> 1.0.0
```

每次发版会：

1. 更新 `package.json`、`src-tauri/Cargo.toml` 和 `src-tauri/tauri.conf.json` 中的版本号。
2. 运行 `scripts/sync-cargo-lock.mjs`，通过 `cargo update` 同步 `src-tauri/Cargo.lock` 里的 `q-note` 条目。
3. 创建 `release: vX.Y.Z` 提交和 `vX.Y.Z` 标签，并推送到 `origin`。

发版脚本使用 `bumpp --all`，这样 `Cargo.lock` 会和版本文件一起进入同一个 release 提交。不要去掉 `--all`，否则 `bumpp` 只会提交它直接修改的版本文件。

推送 `v*` 标签会触发 [`.github/workflows/release.yml`](./.github/workflows/release.yml)，在 Windows、macOS 和 Linux 上构建安装包，发布 GitHub Release，并上传应用内更新所需的 `latest.json`。

## macOS 信任应用

Q Note 目前没有使用 Apple Developer ID 证书签名和公证。macOS 安装后可能会提示“Q Note 已损坏，无法打开”。如果你确认安装包来自官方 GitHub Release，并且信任这个应用，可以手动允许打开：

1. 打开 **系统设置**。
2. 进入 **隐私与安全性**。
3. 在安全性区域找到 Q Note 的拦截提示，点击 **仍要打开**。
4. 也可以右键点击 **Q Note.app**，选择 **打开**，再确认系统提示。

如果 macOS 仍然拦截，可以在终端移除下载隔离标记：

```bash
sudo xattr -rd com.apple.quarantine "/Applications/Q Note.app"
open "/Applications/Q Note.app"
```

请只对你确认来自可信 Release 页面的应用执行这个命令。

## 许可证

本项目使用 [MIT](./LICENSE) 协议。
