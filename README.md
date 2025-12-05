# Clock App

跨平台定时关机 GUI 应用，基于 Rust + Tauri + Leptos。支持倒计时与指定时分关机，提供取消、系统托盘、系统通知与实时倒计时显示。

## 功能

- 倒计时关机与指定时间关机
- 取消关机按钮与托盘菜单操作
- 剩余时间实时倒计时，每秒更新
- 系统托盘图标与菜单
- 关机前 60 秒系统通知提醒
- 日志记录与错误处理

## 开发与运行

- 安装依赖：Rust 稳定版
- 开发启动：`cargo tauri dev`
- 前端构建：`trunk build --release`

## 构建与打包

- 打包命令：`cargo tauri build`
- 产物位置：`src-tauri/target/release/bundle/`
- Windows 产物包含 `msi` 与 `nsis` 安装包

### 一键打包

- 首次构建（自动安装必要依赖后打包）：

```
rustup target add wasm32-unknown-unknown; cargo install trunk; cargo tauri build
```

- 已安装依赖的情况下：

```
cargo tauri build
```

## 使用

- 打开应用后在标题下方查看当前时间（YYYY-MM-DD HH:mm:ss），每秒刷新
- 设置倒计时分钟数或指定时分进行关机计划
- 使用“取消关机”按钮或托盘菜单取消计划

## 技术栈

- 后端：Tauri v2，Rust 2021
- 前端：Leptos CSR，Trunk 构建
- 插件：`tauri-plugin-log`，`tauri-plugin-notification`，`tauri-plugin-opener`

## 测试与质量

- 运行 `cargo clippy` 与 `cargo build` 进行检查
- 单元测试示例位于 `src-tauri/tests/`

## CI

- GitHub Actions 工作流位于 `.github/workflows/release.yml`
- 触发条件：推送匹配 `v*` 标签或手动触发
- 构建平台：`macos-latest`、`windows-latest`
- 产物通过 Artifact 输出到 `src-tauri/target/release/bundle/`

## 权限与平台注意

- Linux 执行关机可能需要系统权限或策略配置
- Windows 通知在开发模式下图标为 PowerShell，安装后恢复为应用图标

## 目录结构

- 根目录：Leptos 前端与构建配置
- `src-tauri/`：Tauri 后端、打包与权限配置
- `styles.css`：全局样式与响应式栅格系统
- `src/app.rs`：前端界面与交互逻辑
