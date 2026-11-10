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
  app.rs          外壳：任务聚合、确认与应用状态
  backend.rs      外壳消费的后端/分组描述
  plugin.rs       插件清单：provides、requires、backend、pages、groups
  plugins/        所有可选单元与注册表
    mod.rs        探测、需求解析、组装
    labwc/        后端插件（requires `wlr.outputs`）：适配器、配置格式、页面
    wlr/          输出提供者插件：拥有 `wlr.outputs` 契约
    wlr_ui/       输出页面插件：适配 `wlr.outputs`
  config/         ConfigFile / ConfigFormat / ConfigEdit / PatchOutcome
  modules/        所有页面共用的 SettingsModule 契约
  pages/          预留的合成器无关页面（目前为空）
  services/       通用文件事务和后台 worker
  core/           领域模型：Effect、Task、ApplyReport
  ui/             侧边栏、状态栏和主题
  i18n.rs         运行时翻译加载
```

依赖方向：

- `core` 与 `config` 不认识插件、页面或 UI。
- 能力属于声明它的提供者，名字带命名空间（`wlr.outputs`）。消费者必须 `require`
  该能力并 import 提供者模块；换提供者意味着把消费者适配到新契约。
- 提供者无 UI，也不 import 其他插件；只有 `plugins/mod.rs` 引用所有插件。
- `plugin_imports_are_backed_by_requires` 检查：import 另一个插件必须有 `requires` 背书。
- `pages/` 不得 import `crate::plugins`，也不得出现后端键空间。
- 注册表只在 `requires` 被满足时启用插件；缺提供者时禁用该插件，而不是半套运行。
- `modules/` 只是页面契约：页面只声明后端文件 id，不出现路径。
- `app` 读取解析后的后端文件清单，按文件聚合任务并分发页面请求，从不指名插件。
- 持久化统一使用 `Effect::WriteFile` / `Effect::RunCommand`。
- `src/architecture_tests.rs` 在测试期强制这些依赖方向。

`LXWC_BACKEND` 可按 id 强制指定后端插件。

## 可选组件

| Feature | 内容 | 默认 |
| --- | --- | --- |
| `labwc` | Labwc 后端插件，自动带上 `wlr` | 开 |
| `wlr` | `wlr-randr` 提供者：拥有 `wlr.outputs` | 开（由 `labwc` 带入） |
| `wlr-ui` | 输出页面插件，requires `wlr.outputs` | 开 |

`cargo build --no-default-features` 构建零插件外壳，界面显示“没有启用任何合成器后端或插件”。
卸载 `wlr` 必须同时卸载 `wlr-ui` 与 `labwc`，依赖边由 Cargo 保证。

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
