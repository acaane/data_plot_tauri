# data_plot_tauri

基于 Tauri + Vue 3 的桌面日志图表工具。

## 开发

```bash
pnpm install
pnpm dev
pnpm tauri dev
```

## 构建

```bash
pnpm build
pnpm tauri build
```

## Ubuntu 打包

Tauri 桌面端默认是在哪个平台构建，就产出哪个平台的安装包。当前如果在 Windows 上执行 `pnpm tauri build`，得到的仍然是 Windows 安装包，不能直接产出 Ubuntu 的 `.deb` 或 `AppImage`。

可选方案：

1. 在 Ubuntu 22.04 上本地构建

```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf
corepack enable
pnpm install
pnpm tauri build -- --bundles deb,appimage
```

2. 使用仓库内的 GitHub Actions 工作流 `build-ubuntu`

该工作流会在 `ubuntu-22.04` runner 上安装依赖并构建 Ubuntu 安装包，产物会作为 workflow artifacts 上传，可直接下载。
