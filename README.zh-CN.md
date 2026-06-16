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

Q Note 基于 Tauri 2、React、TypeScript、Vite+、Tailwind CSS、SQLite、Drizzle 和 dnd-kit 构建。应用面向轻量桌面使用场景：打开快、界面紧凑、卡片列表可扫描，适合保存代码片段、常用回复、图片素材、本地路径和临时截图。

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

| 层级     | 技术                                                  |
| -------- | ----------------------------------------------------- |
| 桌面容器 | Tauri 2                                               |
| 前端     | React 19 + TypeScript                                 |
| 构建     | Vite 8 + Vite+                                        |
| 样式     | Tailwind CSS 4 + CSS                                  |
| 拖拽排序 | dnd-kit                                               |
| 数据     | SQLite + `@tauri-apps/plugin-sql` + Drizzle proxy     |
| 文件     | `@tauri-apps/plugin-dialog` + `@tauri-apps/plugin-fs` |
| 图标     | lucide-react + 黄色 Q 应用图标                        |

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

项目配置了 GitHub Actions。推送 `v*` tag 时会触发 release workflow，在 Windows、macOS 和 Linux 上构建 Tauri 安装包，并发布到对应 GitHub Release。Windows 会产出 NSIS/MSI，macOS 会产出 DMG，Linux 会产出 AppImage、DEB 和 RPM（平台支持时）。

```bash
git tag v0.1.0
git push origin v0.1.0
```

## 许可证

本项目使用 [MIT](./LICENSE) 协议。
