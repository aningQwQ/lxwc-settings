# TODO

当前已完成外观、窗口行为、鼠标/触控板、键盘、触摸屏的标量设置，以及 rc.xml 与环境文件的跨模块聚合写入。以下是尚未实现或与 labwc-tweaks 存在差异的功能。

## P1 自定义指针加速曲线

对应 labwc-tweaks 的 Custom acceleration profile，需要 labwc >= 0.20.3。

- 编辑 `motionCurve`、`scrollCurve`、`fallbackCurve`。
- 每条曲线支持 Define 开关、Step、Points。
- 校验 Step 为有限正数。
- 校验 Points 为 2–32 个有限非负数，分隔符兼容空格、Tab、换行。
- 无效输入阻止 Apply，并显示具体错误。
- 写入时使用子元素 `step`、`points`。
- 取消 Define 时删除对应曲线节点。
- 未修改的曲线保留原 XML 结构和属性。
- 兜底规则：Motion/Scroll 未定义时使用 Fallback；Fallback 也未定义时不启用加速。
- 选定 Custom 时禁用 Pointer Speed，Scroll Factor 仍可用。
- 补充曲线解析、校验、写入和删除测试。

## P1 About / 系统信息页

- 显示 labwc 版本：`LABWC_VER`。
- 显示 `LABWC_PID`。
- 执行 `labwc -v` 并检测特性：
  - `+xwayland`
  - `+nls`
  - `+rsvg`
  - `+libsfdo`
- 从版本输出解析 wlroots 版本。
- 显示 `XDG_CURRENT_DESKTOP`。
- 显示 `WAYLAND_DISPLAY`。
- 显示实际配置目录。
- 显示 labwc-tweaks 版本和项目链接。
- 页面只读，不产生 Apply 任务。

## P1 XKB 选项与布局编辑完善

- 补齐 labwc-tweaks 的完整 `grp:*` 选项预设及中英文翻译。
- 保留自由文本输入，允许任意 XKB option。
- 布局变体改为 `layout(variant)` 形式写入 `XKB_DEFAULT_LAYOUT`。
- 写入带变体的布局时将 `XKB_DEFAULT_VARIANT` 清空。
- 布局下拉框显示代码和描述。
- `evdev.lst` 不存在时提供更完整的回退布局列表。
- 增加布局解析、变体合并和序列化测试。

## P2 UI 行为对齐

- Pointer Speed 使用与 labwc-tweaks 一致的 `-100..100` 刻度，底层除以 10 写入 float。
- 主题列表在扫描结果前补充 `Adwaita` 回退项。
- 确认所有 ComboBox 在自定义值不在候选列表时仍能正确显示。

## P2 动态显示持久化

- 目前 WLR 默认显示模块只支持静态 `autostart` 写入。
- 动态 kanshi profile 仍标记为“暂未实现”。
- 如后续支持，需要设计 profile 生成、文件路径、命名和回滚策略。

## P3 开发者模板页

- labwc-tweaks 的 Template 页面仅用于开发示例。
- 暂不移植；如需要，可在调试开关后提供示例模块。

## 测试与验证

- 为跨模块聚合增加测试：多个模块同时修改 rc.xml 和环境文件时只生成一个文件 Task。
- 验证快捷键补丁与标量补丁可以合并。
- 验证环境文件新增多个变量时只产生一个 `# added by lxwc-settings` 区块。
- 使用真实 `~/.config/labwc/rc.xml` 做只读补丁校验。
- 每次功能完成后运行：

```bash
cargo fmt
cargo build --offline
cargo test --offline
cargo build --release --offline
```
