<p align="center"><img src="./app-icon.png" alt="Q Note" width="100" /></p>

# Q Note · NSH 版

[English](./README.md)

使用 [Native Shell（nsh.dev）](https://nsh.dev) 的原生组件与 TypeScript 实现。保留项目黄色主题、12 色便签、独立编辑窗、系统托盘及 30px 悬浮球。

当前为 `nsh` 分支，完整替换原 Vue/Tauri 实现，兼容现有 SQLite 数据库和 JSON 导入导出格式。

## 开发

需要 Node 24、pnpm 12 和 Rust stable。Windows 需要 Visual Studio C++ 构建工具；Linux 需要 GTK 4.14+ 和 clang。

```sh
pnpm install
pnpm dev
```

首次运行会编译应用的 Rust 扩展与原生宿主，后续复用编译缓存。

| 命令 | 用途 |
| --- | --- |
| `pnpm dev` | 原生桌面应用 |
| `pnpm dev:web` | 可选网页预览 |
| `pnpm typecheck` | 生成原生声明并检查类型 |
| `pnpm test` | 数据与逻辑回归 |
| `pnpm test:ui` | NSH 界面回归，使用隔离数据 |
| `pnpm test:desktop` | Windows 原生截图、无边框尺寸、独立编辑窗及悬浮球验证 |
| `pnpm test:native` | 数据库迁移、回滚和更新取消测试 |
| `pnpm check` | 类型与格式检查 |
| `pnpm format` | 格式化 TypeScript/TSX |
| `pnpm build` | 编译应用字节码 |
| `pnpm package:windows` | Windows 便携应用和 ZIP |
| `pnpm package:macos` | macOS 应用及压缩包 |
| `pnpm package:linux` | Linux 应用目录及 tar.gz |
| `pnpm package:web` | 静态网页预览 |

产物位于 `dist/<平台>`。网页预览使用独立浏览器存储和页内编辑器；托盘、悬浮窗、本地文件路径、开机启动及桌面导入导出需要原生桌面应用。

可选网页运行时还需要 `wasm32-unknown-unknown` Rust target，以及 PATH 中可用的 LLVM/clang。

## 固定 NSH 版本

多窗口和托盘需要 nightly 提交 `c5214aaf4bbdf33bc1d59e9595e009b6aa54d702`。启动脚本会下载官方包并验证 SHA-256，也支持相同版本的全局 NSH 或 `Q_NOTE_NSH_BIN`。

如果官方滚动 nightly 已更新，脚本会自动从固定提交编译 CLI。也可手动编译：

```sh
git clone https://github.com/longbridge/nsh nsh-toolchain
git -C nsh-toolchain checkout c5214aaf4bbdf33bc1d59e9595e009b6aa54d702
cargo build --release --manifest-path nsh-toolchain/Cargo.toml -p nsh-cli
```

将 `Q_NOTE_NSH_BIN` 设置为生成的 `target/release/nsh.exe`（Windows）或 `nsh`（macOS/Linux）绝对路径。源码检出会自动编译所需宿主。不同提交的运行时会被拒绝，避免组件 API 静默变化。

## 功能和数据

支持文本、内嵌图片、图片地址、本地路径，便签置顶、改色、复制、逐行调整高度及跨分组排序。编辑器支持图片选择、文件拖入、截图粘贴；图片预览支持键盘操作、缩放和平移。界面提供中文和英文。

数据库仍位于 `~/.q-note/q-note.db`。新文件不存在时自动迁移旧配置目录中的数据库，包含已提交的 WAL 数据，并保留旧文件。手动验证可设置 `Q_NOTE_DATA_DIR` 指向隔离目录。

完整产品行为见[功能规格](./docs/FEATURES.md)，架构与验证方式见[NSH 实现说明](./docs/NSH.md)。

修改 Windows 窗口、托盘、动画或更换桌面技术栈前，先读 [Windows 已修复问题与回归约束](./docs/WINDOWS_REGRESSIONS.md)。其中记录“切球后不可见、打开托盘菜单才出现”的故障、已确认修复及以后必须保留的首显检查；详细用例使用 main 的[迁移测试基线](https://github.com/ywenhao/q-note/blob/main/docs/TEST_CASES.md)，包含 DOCK-022–025。

## 发布

`pnpm release:patch --dry-run` 预览版本变更。patch/minor/major 命令同步所有版本、运行检查、创建提交和 `nsh-vX.Y.Z` 标签并推送，只有准备发布时才执行。

GitHub Actions 构建各桌面平台、生成带校验信息的 NSH 更新清单并发布独立版本。应用只检查 `nsh-v*`，支持真正中断下载，并使用 NSH 的校验安装器；原版的 latest 发布保持独立。公开分发的 macOS 包需要 Developer ID 签名及公证。

[MIT](./LICENSE)
