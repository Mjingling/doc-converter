# EP02 桌宠降临：透明置顶窗口

> 系列第 2 篇 | Demo：`ep02/demo` | 预计阅读 15 分钟

本篇目标：屏幕右下角出现一个 150×180 的小窗口，**透明、无边框、永远置顶**，
里面站着一个静态机器人。这是桌宠的"房子"。

## 运行本篇 Demo

```bash
cd tutorials/ep02/demo
npm install
npm run tauri dev
```

看到桌面右下角浮出机器人即成功。它没有标题栏、没有背景色、点不动也关不掉
（本集还没做交互，`Cmd+Q` / 任务栏退出进程即可）。

## 第一步：Tauri 项目骨架

一个最小 Tauri 2 应用长这样：

```
demo/
├── package.json            # 前端：vue + @tauri-apps/api
├── vite.config.ts          # Vite 开发服务器（端口 5182）
├── index.html              # 前端入口
├── src/
│   ├── main.ts             # 挂载 Vue
│   └── App.vue             # 机器人 SVG
└── src-tauri/
    ├── Cargo.toml          # Rust 依赖：tauri
    ├── build.rs            # tauri_build::build() 固定写法
    ├── tauri.conf.json     # 窗口与应用配置（本篇主角）
    ├── capabilities/       # 权限声明（Tauri 2 安全模型）
    └── src/main.rs         # Rust 入口（本篇主角）
```

关键依赖就三个：前端 `@tauri-apps/api`（IPC 客户端）、
Rust 侧 `tauri` + `tauri-build`。

## 第二步：声明一个"非主流"窗口

桌宠窗口的所有"反常规"属性，全部在 `tauri.conf.json` 里声明：

```json
"app": {
  "windows": [
    {
      "label": "pet",
      "title": "桌宠",
      "width": 150,
      "height": 180,
      "decorations": false,
      "transparent": true,
      "alwaysOnTop": true,
      "resizable": false,
      "skipTaskbar": true,
      "shadow": false
    }
  ],
  "macOSPrivateApi": true
}
```

逐项解释：

| 属性 | 作用 |
|------|------|
| `decorations: false` | 去掉标题栏和边框 |
| `transparent: true` | 窗口背景透明（桌宠的灵魂） |
| `alwaysOnTop: true` | 永远浮在其他窗口之上 |
| `skipTaskbar: true` | 不出现在任务栏 / Dock 切换器里 |
| `shadow: false` | 关掉窗口阴影，否则会看到一圈灰边 |
| `macOSPrivateApi: true` | **macOS 上透明窗口依赖私有 API，必须开** |

对应地，Cargo 里要启用同名 feature，否则 macOS 编译能过但运行不透明：

```toml
tauri = { version = "2", features = ["macos-private-api"] }
```

> 坑提醒：`transparent` 窗口在 Linux 部分合成器下表现不一，
> Windows 11 正常。本系列以 macOS / Windows 为主。

## 第三步：前端也得透明

窗口透明只是"玻璃"，页面内容默认还是白纸。`App.vue` 里必须手动清背景：

```ts
document.documentElement.style.background = "transparent";
document.body.style.background = "transparent";
```

漏掉这一步的效果：桌面上漂着一块白色矩形——第一版我就犯了这个错。

机器人本体先放一个静态 SVG（天线 + 方头 + 屏幕脸），50 行以内，
EP03 会把它换成会眨眼的版本。

## 第四步：把窗口摆到屏幕右下角

重头戏。要求：**主显示器工作区的右下角，距离两边各留 24px**。

三个细节决定了实现方式：

1. **工作区 ≠ 屏幕尺寸**。macOS 有菜单栏、Windows 有任务栏，
   要用 `work_area()` 而不是 `size()`
2. **HiDPI 缩放**。Retina 屏上 1 逻辑像素 = 2 物理像素，
   Tauri 的 monitor API 返回物理像素，而窗口定位用逻辑像素，
   中间必须换算
3. **可测试性**。坐标计算抽成纯函数，单测伺候

`src-tauri/src/main.rs` 核心代码：

```rust
const PET_W: f64 = 150.0;
const PET_H: f64 = 180.0;
const PET_MARGIN: f64 = 24.0;

/// 工作区（物理像素）+ 缩放 → 窗口左上角的逻辑坐标
fn bottom_right_position(area_x: f64, area_y: f64, area_w: f64, area_h: f64, scale: f64) -> (f64, f64) {
    let scale = if scale <= 0.0 { 1.0 } else { scale };
    let px = area_x + area_w - PET_W * scale - PET_MARGIN * scale;
    let py = area_y + area_h - PET_H * scale - PET_MARGIN * scale;
    (px / scale, py / scale)
}
```

然后在应用 `setup` 阶段取主显示器并应用：

```rust
.setup(|app| {
    let win = app.get_webview_window("pet").expect("宠物窗口不存在");
    if let Ok(Some(m)) = win.primary_monitor() {
        let wa = m.work_area();
        let (x, y) = bottom_right_position(
            wa.position.x as f64, wa.position.y as f64,
            wa.size.width as f64, wa.size.height as f64,
            m.scale_factor(),
        );
        let _ = win.set_position(LogicalPosition::new(x, y));
    }
    Ok(())
})
```

注意 `wa.position`：多显示器时副屏的工作区坐标可能为负值或大于主屏宽度，
公式里带上 `area_x / area_y` 天然兼容。

## 第五步：单测锁死坐标逻辑

```rust
#[test]
fn test_1x_scale() {
    let (x, y) = bottom_right_position(0.0, 0.0, 1440.0, 900.0, 1.0);
    assert_eq!((x.round(), y.round()), (1266.0, 696.0));
}

#[test]
fn test_retina_2x() {
    // 物理 2880×1800、2x 缩放 → 逻辑结果与 1x 完全一致
    let (x, y) = bottom_right_position(0.0, 0.0, 2880.0, 1800.0, 2.0);
    assert_eq!((x.round(), y.round()), (1266.0, 696.0));
}
```

运行：

```bash
cd src-tauri && cargo test
```

> 成品 DocMorph 里这段逻辑在 `src-tauri/src/commands/pet.rs`，
> 只是从 setup 挪进了 `pet_show` 命令（因为成品的宠物是设置里开关的，
> 需要运行时动态创建窗口），公式一字未改。

## 本篇小结

| 知识点 | 一句话 |
|--------|--------|
| 透明无边框窗口 | `transparent + decorations:false`，macOS 要开 `macOSPrivateApi` |
| 页面背景 | 窗口透明后必须手动清 `html/body` 背景 |
| 屏幕定位 | `work_area`（不是屏幕尺寸）+ 缩放换算 + 纯函数可测 |

## 下一篇预告

[EP03 让宠物活起来](../ep03/article.md) ——
静态机器人太呆了。下一篇给它装上眼睛（随机眨眼）、睡眠模式（Zzz 气泡）、
悬浮呼吸感，并建立"状态机 + 随机调度"的行为框架。
