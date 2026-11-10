# lxwc-settings

[English](README.md) | 简体中文

一个使用 Rust 和 egui 编写的 Labwc + LXQt 轻量设置中心。

`lxwc-settings` 用于编辑 Labwc 的 `rc.xml`、`environment` 和 `autostart`，
并提供 Labwc 配置文件本身没有覆盖的 WLR 输出控制。所有修改都会先校验，
再备份并原子写入。

## 功能

- **显示与输出（WLR）**  
  使用 `wlr-randr` 扫描输出和模式、切换模式；15 秒内未确认时自动回滚。

- **全局快捷键**  
  在 `rc.xml` 中查看、添加、编辑和删除 Labwc 快捷键，支持按键组合、动作、
  属性以及高级 XML 片段。

- **默认显示配置**  
  将选定的输出和模式作为 `wlr-randr` 命令写入 Labwc 的 `autostart`。

- **外观**  
  窗口主题、图标主题、圆角、阴影、装饰模式、最大化装饰。

- **窗口行为**  
  放置策略、焦点、窗口间距、吸附、边缘阻力、缩放和放大镜。

- **鼠标与触控板**  
  libinput 设置、光标主题与大小，以及自定义指针加速曲线
  （`motionCurve`、`scrollCurve`、`fallbackCurve`）。

- **键盘**  
  XKB 布局、变体、切换选项、重复速率/延迟和 NumLock。

- **触摸屏**  
  校准矩阵选择。

- **环境变量**  
  串流与兼容性开关，例如 `WLR_DRM_NO_MODIFIERS` 和
  `WLR_NO_HARDWARE_CURSORS`。

- **LXQt 预设**  
  提供桌面集成使用的预设命令。

- **安全机制**  
  所有写入都经过校验和统一文件事务：`.lxwc.bak` 备份、原子替换、
  外部修改检测、失败任务重试。

- **国际化**  
  英文和简体中文翻译从外部 `locales/*.lang` 文件加载。

## 依赖

- 运行 Labwc 的 Linux Wayland 会话
- 较新的稳定 Rust 工具链（edition 2021）
- `wlr-randr`：扫描输出和切换模式
- `labwc`：重新加载配置
- 可选：`fc-match`/fontconfig，用于自动加载中文字体

## 构建与运行

```bash
cargo build --release
./target/release/lxwc-settings
```

开发运行：

```bash
cargo run
```

从源码安装：

```bash
cargo install --path .
```

## 配置路径

默认读取和写入：

- `~/.config/labwc/rc.xml`
- `~/.config/labwc/environment`
- `~/.config/labwc/autostart`

可通过以下环境变量覆盖：

| 变量 | 作用 |
| --- | --- |
| `LXWC_CONFIG_DIR` | Labwc 配置目录 |
| `LXWC_RC_XML` | `rc.xml` 路径 |
| `LXWC_ENVIRONMENT` | `environment` 路径 |
| `LXWC_AUTOSTART` | `autostart` 路径 |
| `LXWC_LANG` | 强制使用 `zh` 或 `en` |
| `LXWC_LOCALE_DIR` | 额外翻译目录 |
| `LXWC_XKB_RULES` | 替代的 `evdev.lst` 路径 |

修改 `rc.xml` 后会触发 Labwc 重新加载配置。环境变量修改需要新会话生效。

写入前会将原文件复制为 `.lxwc.bak`，临时文件使用 `.lxwc.tmp` 后缀。

## 项目结构

```text
src/
  app.rs          后端驱动的任务聚合、确认与应用状态
  backend.rs      外壳消费的后端/分组描述
  backends/       每个合成器一个目录：适配器、配置格式、页面
    labwc/        rc.xml、environment、autostart、XKB、快捷键与 Labwc 页面
    mod.rs        探测、工厂、页面注册
  config/         ConfigFile / ConfigFormat / ConfigEdit / PatchOutcome
  modules/        所有页面共用的 SettingsModule 契约
  pages/          与具体合成器无关的页面
  wlr/            WLR 适配器：wlr-randr 扫描、解析、模式命令
  plugins/        无 UI 的预设，例如 LXQt 命令
  services/       通用文件事务和后台 worker
  core/           领域模型：Effect、Task、TaskResult、ApplyReport
  ui/             侧边栏、状态栏和主题
  i18n.rs         运行时翻译加载
```

依赖方向：

- `core` 与 `config` 不认识页面、UI 或具体合成器。
- `backends/<id>` 拥有路径、文件格式、重载命令和它自己的页面。
- `pages/` 存放合成器无关页面，不得 import `backends/`。
- `modules/` 只是页面契约：页面只声明后端文件 id，不出现路径。
- `app` 读取后端文件清单，按文件聚合任务，并分发页面请求。
- 持久化统一使用 `Effect::WriteFile` / `Effect::RunCommand`。
- `src/architecture_tests.rs` 在测试期强制这些依赖方向。

`LXWC_BACKEND` 可强制指定后端；目前只有一个后端时默认 `labwc`。

## 测试

```bash
cargo fmt --check
cargo build
cargo test
cargo build --release
```

测试覆盖 XML 补丁、快捷键、环境文件、标量设置、主题、XKB 布局解析、
加速曲线、显示解析和文件事务安全。

可选的真实配置校验：

```bash
LXWC_TEST_RC="$HOME/.config/labwc/rc.xml" cargo test real_config
```

## 文档

- 未完成工作：[`doc/TODO.md`](doc/TODO.md)
- 开发守则：[`doc/守则.md`](doc/守则.md)
- 翻译文件：[`locales/`](locales/)

## 许可证

GPL-3.0-or-later，详见 [`LICENSE`](LICENSE)。
