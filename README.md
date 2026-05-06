# clash-verge-portable

面向 Windows 便携场景的 Clash Verge 分支。

## 本分支相对原版的主要改动

- 便携版统一使用 `Data/` 目录：配置与状态写入 `Data/app/`。
- 优化 Windows WebView2 数据目录与回退逻辑，减少便携版白屏问题。
- 安装器的 WebView2 改为显性提示（非静默安装）。
- 修正 Windows 系统代理应用顺序，避免“开启后自动回退”。
- 新增后端直连 IP 查询（`panel_http_request`），提升查询稳定性。
- Windows 下窗口状态写入 `Data/app/window_state.json`，便携模式不落盘到 `%APPDATA%`。
- 更新便携打包脚本，兼容不同 release 产物目录并生成 zip。

## Development

```shell
pnpm i
pnpm run prebuild
pnpm dev
```

## Build And Release (Windows)

常用命令（在项目根目录执行）：

```shell
pnpm build
pnpm portable
```

- `pnpm build` 生成安装包（NSIS）到 `target/release/bundle/nsis/`
- `pnpm portable` 生成便携压缩包到项目根目录（`Clash.Verge_<version>_<arch>_portable.zip`）
- 便携版数据目录使用 `Data/` 结构，配置与状态默认写入 `Data/app/`

## License

GPL-3.0 License. See [License here](./LICENSE) for details.
