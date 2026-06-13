# Q Note

> 一个小而快的桌面便签面板，用来存放经常复制的内容。

[English README](./README.en.md)

## 项目定位

Q Note 是基于 Tauri 2、React、TypeScript、Vite+、Tailwind CSS、SQLite 和 Drizzle 的桌面便签应用。它适合保存短文本、图片、网络图片地址、本地图片路径、普通文件路径和截图粘贴内容，并提供快速复制、窗口置顶、Q 图标模式、颜色标记、导入导出和全部删除确认。

## 快速开始

```bash
pnpm install
pnpm tauri dev
```

## 常用命令

| 命令                | 说明                 |
| ------------------- | -------------------- |
| `pnpm dev`          | 启动 Vite 开发服务   |
| `pnpm tauri dev`    | 启动 Tauri 桌面应用  |
| `pnpm typecheck`    | 运行 TypeScript 检查 |
| `pnpm check`        | 运行 Vite+ 检查      |
| `pnpm check:fix`    | 自动修复检查问题     |
| `pnpm format`       | 使用 Vite+ 格式化    |
| `pnpm format:check` | 检查格式             |
| `pnpm build`        | 构建前端             |

## 功能概览

| 功能       | 说明                                                                              |
| ---------- | --------------------------------------------------------------------------------- |
| 中英文切换 | 顶部语言按钮可切换中文和英文，并保存到本地                                        |
| 快速复制   | 点击卡片默认复制文本；纯附件便签会复制附件值                                      |
| 卡片管理   | 支持新建、编辑、删除、置顶、颜色标记和高度拖拽                                    |
| 全部删除   | 列表支持全部删除，并通过红色确认弹窗二次确认                                      |
| 图片与文件 | 编辑器支持选择图片、拖入文件/图片、粘贴截图、网络图片 URL、本地路径和 base64 兜底 |
| 大图预览   | 编辑器中的图片缩略图可点击查看大图                                                |
| 窗口置顶   | 顶部图标或右键菜单可切换窗口最高层级                                              |
| Q 图标模式 | 点击折叠图标变成黄色 Q 图标；Q 图标可拖动、贴边吸附、悬停展开                     |
| 数据持久化 | 便签、附件、颜色、卡片高度、窗口大小、置顶状态和语言保存到 SQLite                 |
| 导入导出   | 支持导入导出便签和本地配置 JSON                                                   |

## 技术栈

| 层级     | 技术                                                  |
| -------- | ----------------------------------------------------- |
| 桌面容器 | Tauri 2                                               |
| 前端     | React 19 + TypeScript                                 |
| 构建     | Vite 8 + Vite+                                        |
| 样式     | Tailwind CSS 4 + CSS                                  |
| 数据     | SQLite + `@tauri-apps/plugin-sql` + Drizzle proxy     |
| 文件     | `@tauri-apps/plugin-dialog` + `@tauri-apps/plugin-fs` |
| 图标     | lucide-react + 黄色 Q 应用图标                        |

## 更多说明

完整中文说明见 [read.md](./read.md)。

## 许可证

[MIT](./LICENSE)
