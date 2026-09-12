# `window_controls_demo` 红绿灯状态机修复 Plan

## 执行结果（已完成）

**决策**（用户确认）：
1. 拖出按钮时**撑着直到松手**（保留 `750ab52` 的语义）；
2. 峰值**略带回弹**（ζ = 0.70，bounce 0.30，峰值 ≈ 1.188）；
3. 清理并落地为 window-shell repo 内的**独立 crate**；
4. liquid-rs 侧只保留 `TrafficLightStyle` / `GlassVariant` / shader，**tuning 留在 shell 红绿灯 crate**。

**落地**：新建 `crates/bmol-window-traffic-lights`（9 个模块），删除 `crates/bmol-window-glass`（其内容全部并入新 crate）。

| 模块 | 职责 |
|---|---|
| `state.rs` | 交互状态机（无 Iced 依赖），`REST=1.0` / `PEAK=1.18`，`armed`，真实 dt |
| `interaction.rs` | 单一交互帧（`publish_group` 写入 / `snapshot` 读取），消除重复平滑 |
| `palette.rs` / `layout.rs` / `tuning.rs` | 调色板、度量、材质调参 |
| `scene.rs` | `GlassScene` 构建器（给共享同一 liquid-glass-scene 版本的消费者） |
| `widget.rs` | `TrafficLightGroup`：**整个组一个 widget** 负责 hover 揭示、带 slop 的逐控件命中测试、按下生命周期 |

**关键修复**：
- `PRESS_SCALE_SETTLED=1.06` 删除 → 静止值恒为 1.0，峰值受 `bounce` 控制（新增单测卡住峰值/静止值/无下冲）。
- 拖出只清 tint、尺寸保持到松手；松手无条件收敛并清 `armed`（含丢失 release 的 `release_armed()` 兜底）。
- `interactive` 与"执行窗口命令"解耦：5 组样例全部有按下动画，只有 Native 组执行命令。
- 5 组各自持有一个 `TrafficLightsState`，删除 demo 的并行数组与两套索引算术。
- 焦点跟踪（`Focused`/`Unfocused`）与脏文档点（显式 `close_dot`）进状态机，不再依赖进程级 static 假装。
- 合成器不再二次平滑 hover/press。

**验证**：`cargo test --workspace` 62 项全绿（新 crate 29 项）；新 crate clippy 零警告；`cargo build --examples` 通过。

**第二轮补充**：`layout.rs` 现在以 `WINDOW_CONTROL_*_RAW_IDS` / `WINDOW_CONTROL_RAW_GROUPS`（`u64`）为 id 的唯一真源，typed `GlassId` 由 `const fn typed_ids` 派生；`interaction.rs` 增加与版本无关的 `slot_index_for_raw_id` / `group_index_for_raw_id`。`iced_backend.rs` 删掉了自己那份 group/slot 映射表，改为 `local_ids(RAW_IDS)` + 调用 crate 的 raw 查询 —— 即使两边的 `liquid-glass-scene` 版本不同，id 表和索引逻辑也不再重复。新增单测锁定 raw 表与 typed id 的一致性以及 id 唯一性。

**第三轮补充（消除"配方"重复）**：把与 glass 类型无关的决策逻辑下沉到 crate，使其成为唯一真源 ——
- `WindowControlTuning::material_fields()` → `TrafficLightMaterialFields` / `TrafficLightStyleFields`：不活跃窗口的 muted alpha/opacity 公式、以及 12 个物理 style 旋钮的映射只写一次；两个 consumer 只做"数字→自家类型"的机械拷贝。
- `scene::sphere_bounds()`：球体放大时的 bounds 公式（scale + 居中 inset）只写一次，附带"邻居不位移"的单测。
- `palette::blend_rgba()`：`blend_color` 变成它的类型化薄包装。
- `iced_backend` 里 `traffic_light_material` / `push_traffic_light_group` / `blend_color` 现在只是类型适配；同时删掉了 `AnimatedInteraction.last_updated`（渲染端旧动画时钟的残留）。

**第四轮：版本对齐完成（阻塞解除）**

以 GitHub 远端为准核对（`git ls-remote`，不是本地 checkout）：

| 仓库 | GitHub 最新 | 说明 |
|---|---|---|
| liquid-rs | tag **v0.1.3** | scene/render/geometry 均 0.1.3 |
| bmol-designs | tag **v0.1.5** | shell 已用 |
| bmol-iced | tag v0.1.4，但 `main`@3e15508 更新 | v0.1.4 tag 依赖 scene **v0.1.1**；`main` 已依赖 **v0.1.3** |

shell 的两个 bmol-iced dev-dep 从 `tag = "v0.1.4"` 切到 `branch = "main"`，`cargo update` 后 **v0.1.1 系的 liquid-rs/liquid-glass-scene/render/geometry/ui 全部被移除**，锁文件里只剩一个 `liquid-glass-scene 0.1.3`。bmol-iced 新版对本仓库示例无 API 破坏（`cargo check --examples` 直接通过）。

类型对齐后，`iced_backend.rs` 里的适配层全部删除：本地 `GlassId` 包装、id 索引表、`window_control_interaction`、`push_traffic_light_group`、`traffic_light_material`、`traffic_light_color*`、`blend_color`、`scale_scene` —— 全部改为使用 crate 的同一实现。**iced_backend 从 2011 行降到 1559 行（-452）**，`window_controls_demo` 也改为直接从 crate 取 id，不再绕经 demo 自己的渲染模块。

**调色板漂移一并修正**：commit `750ab52`（"fix press colors"）把 glass crate 的 source colours 从
`(1.00,0.34,0.28)/(1.00,0.72,0.05)/(0.18,0.84,0.10)` 改为
`(0.98,0.34,0.30)/(0.98,0.72,0.14)/(0.16,0.77,0.22)`，而同一提交里 `iced_backend.rs` 只改了 size/gap 常量 —— demo 一直渲染的是**该提交漏改的旧值**。现在统一到 crate 的新值，属于修正漂移而非改变既定视觉。

**原"未完成"项现在已全部完成**：`iced_backend.rs` 仍保留它自己的 `GlassScene`/`GlassMaterial`/调色板构造（约 120 行）。原因是在本仓库的 example 依赖图里存在**两个 `liquid-glass-scene` 版本**：`bmol-iced v0.1.4`（tag）仍依赖 liquid-rs **v0.1.1**，而新 crate 与 shell dev-deps 用 liquid-rs **v0.1.3**，`GlassId`/`Color`/`GlassMaterial` 是不同具名类型。要彻底删掉这份重复，需要发布一个依赖 liquid-rs v0.1.3 的 `bmol-iced` 新 tag（本地工作区已经是 v0.1.3，只是 tag 未更新），然后 bump shell 的 dev-dep。届时 `iced_backend` 可直接调用 crate 的 `scene`/`palette`，并删掉本地的 id 索引表。


> 目标：把「红绿灯交互」收敛成**一个**状态机（库里的 `TrafficLightsState`），让 hover / press / cancel /
> release / scale 动画 / 焦点 / 脏文档点只有一个真值来源，并把 demo 里那份分叉出来的并行状态删掉。
> 视觉材质（liquid glass、配色、字形）不动。
>
> 复现命令：`cargo run --example window_controls_demo`（或 `cargo run --bin window_controls_demo`）。
> workspace 里没有名为 `liquid-glass-window-controls-demo` 的 target，这个字符串现在只出现在
> `window_controls_demo.rs:522` 的 “Copy configuration” 文本里。

---

## 0. 结论（TL;DR）

`window_controls_demo` 的交互坏了不是因为视觉，而是因为**同一个交互被三套状态机同时拥有**：

| # | 拥有者 | 位置 | 它管什么 |
|---|--------|------|----------|
| A | 库状态机 `TrafficLightsState` | `src/iced_ui/traffic_lights.rs:203-365` | hover 进度、press tint、press 弹簧、real-dt `step()` |
| B | demo 自建并行数组 | `examples/window_controls_demo.rs:107-118`（`hover_targets[5]` / `hover_progress[5]` / `press_targets[15]` / `press_progress[15]` / `press_springs[15]`） | 5 组 × 3 键的 hover/press/scale，自己再实现一遍指数动画 |
| C | widget 私有 bool | `src/iced_ui/traffic_lights.rs:618-622`（`TrafficLightButtonState { hovered, pressed }`） | 决定“这次点击算不算数”（Action 是否发出） |
| D | 合成器内部平滑 | `examples/playground/iced_backend.rs:402-467`（`glass_interaction`） | 又一次 hover/press 指数平滑，然后被 B 的值覆盖 |

A 在 demo 里**完全没被使用**（只 `pub use` 了名字），B/C/D 各自实现一部分，所以：

- 视觉决策（scale / tint）来自 **B**；
- 行为决策（是否触发窗口命令）来自 **C**；
- GPU 材质数值来自 **B → D**；
- 三者的时间步长、目标值、取消语义各不相同 → hover / 点击 / 放大动画全部表现为“有时候对、有时候不对”。

---

## 1. 缺陷清单

### P0-1 放大后永不回到 1.0（就是“放大动画有问题”）

- 现象：点一下任意按钮，红绿灯**永久变大 6%**；连点会看到峰值远超预期。
- 根因：`examples/window_controls_demo.rs:793` 在 release 时 `retarget(PRESS_SCALE_SETTLED)`，
  而 `PRESS_SCALE_SETTLED = 1.06`（`src/iced_ui/traffic_lights.rs:69`）。它被当作**静止值**，
  所以松手后的 rest scale 就是 1.06 而不是 1.0。
  同时 `PRESS_SCALE_OVERSHOOT = 1.18` 被当作**弹簧目标**（`window_controls_demo.rs:195`），
  而弹簧参数是 `Spring::bouncy_custom(0.24, 0.40)` → `bounce = 0.30 + 0.40 = 0.70`
  → `ζ = 0.30`、`ω₀ = 31.68 rad/s`、解析超调系数 `exp(-πζ/√(1-ζ²)) = 0.372`。
  数值仿真（semi-implicit Euler，60 Hz，与 `spring_rs` 一致）：

  | 场景 | 峰值 | 静止值 | 稳定时间 |
  |------|------|--------|----------|
  | 现在（press→1.18，release→1.06），点击 100 ms | **1.246×**（14 pt → 17.44 pt；64 pt → 79.7 pt） | **1.060×** | 717 ms |
  | 现在（按住 300 ms） | 1.246× | 1.059× | 817 ms |
  | 期望（press→1.18 临界阻尼） | 1.180× | — | ~240 ms |
  | 期望（release→1.0，同一 bouncy 弹簧） | — | 1.000× | 但**下冲 0.919×**（会先缩小再回来） |

  三个独立毛病：**目标语义错**（1.18 是峰值不是目标）、**静止值错**（1.06）、**release 用错弹簧**
  （应临界阻尼，否则下冲）。
- 另外：`src/iced_ui/traffic_lights.rs:6` 的模块文档把 “overshoot 1.18, settle 1.06” 写成了规格，
  这条错误契约需要一起改。

### P0-2 库与 demo 的 release 语义互相矛盾

- 库 `on_press_end` → `retarget(1.0)`（`traffic_lights.rs:287`）
- demo `finish_control_press` → `retarget(1.06)`（`window_controls_demo.rs:793`）
- 同一个 widget 在 `window_demo` 里回到 1.0，在 `window_controls_demo` 里停在 1.06。
- `on_press_cancel`（`traffic_lights.rs:294-300`，来自 commit `750ab52`）只清 tint、**不动弹簧**，
  于是“拖出按钮 → 颜色灭了但尺寸还撑着”直到松手，再落到 1.06。
  `window_demo.rs:409-419` 甚至额外写了一次 `set_window_control_scale(id, 1.0)` 去骗 GPU，
  造成 **CPU 字形 scale(1.18) 与 GPU 球体 scale(1.0) 不同步**。

### P0-3 5 组样例里只有 1 组会做按下动画

- `traffic_lights.rs:1219/1247/1275` 用 `interactive` 同时决定两件事：
  1. 是否挂 press 生命周期回调（`view_single_button_interactive` vs `view_single_button`）；
  2. （在 demo 里）是否真的执行窗口命令（`window_controls_demo.rs:838` 的 `execute: interactive`）。
- demo 只在 Native 组传 `interactive = true`（`window_controls_demo.rs:385`），
  于是 Reference / Large / Inactive / Disabled 四组**没有 press 回调 → 弹簧不动 → GPU press 进度恒为 0**，
  点下去只有底部文字变化。这与该 demo 自己声明的目的（“enlarged active/inactive samples for inspecting
  hover and **press light**”）直接冲突，也是最容易被感知成“hover/点击逻辑坏了”的地方。
- 正解是拆开：**`interactive`（动画）`!=` `executes`（执行命令）**。

### P0-4 CPU 层的 press 高亮被写死为 `false`

- `traffic_lights.rs:1228 / 1256 / 1284` 把 `view_single_button_interactive(..., is_pressed, ...)`
  的第 7 个实参硬编码成 `false`，只有 `view_traffic_lights_all_inclusive`（`traffic_lights.rs:1344` 等）
  才传 `press_targets[i] > 0.5`。
- 结果：GPU 球体有 press tint，CPU 字形层永远没有；两条渲染路径的 press 状态定义不一致。
- 同时 demo 计算出的 `press_progress[15]`（`window_controls_demo.rs:238-247`）**从未进入 CPU widget**，
  只被推给 backend。

### P1-1 hover 被写两遍，且 5 组各自为政

- demo 同时调用 `set_window_control_group_hover(idx, bool)`（`:223`）和
  `set_window_control_group_progress(idx, f32)`（`:249`）；
  backend `glass_interaction` 先把 bool 当 target 平滑一次，再用 progress **直接覆盖** `current.hover`
  （`iced_backend.rs:443-445`）。bool 那条路径实际是死代码，但会参与 `target.hover.max(group_hover)`。
- `hover_targets` / `hover_progress` 是 `[f32; 5]`，与 `press_targets[15]` 用完全不同的索引口径，
  靠 `group.index() * 3`（`window_controls_demo.rs:804`）+ `window_control_slot_index`
  （`:783`）两套算术维持一致，非常脆。
- `TrafficLightsState` 只有一个 group 的 hover（`traffic_lights.rs:206`），无法表达多组 → 这正是 demo
  另起炉灶的原因。

### P1-2 时间步长不真实

- demo 的 hover/press 指数用**硬编码 0.016 s**（`window_controls_demo.rs:234`、`:244`），
  弹簧用**硬编码 1/60**（`:259`）；
- 库 `TrafficLightsState::step(now)` 用的是真实 `dt`（`traffic_lights.rs:302-313`）。
- 在 120 Hz 屏 / 掉帧 / 首帧（`last_tick=None` 走 0.016 兜底）下，动画速度与真实时间不一致，
  且 hover 与 scale 的进度会互相漂移。

### P1-3 焦点与“脏文档点”不在状态机里

- demo 的 `State`（`:107-118`）**没有任何 window focus 字段**，`IcedWindowController::observe`
  只是把事件吞掉了（`:211`）。于是 Native 组永远按“聚焦窗口”渲染，Inactive 组只能是静态假象，
  `is_focused || hover > 0.05` 这条真值（`traffic_lights.rs:1334`）在 demo 中失效。
- `DOCUMENT_EDITED` 是进程级 global static（`traffic_lights.rs:104-117`），demo 从不设置它，
  所以标着 “Running · close status dot” 的样例（`window_controls_demo.rs:466`）实际渲染的是
  一个**淡色 ✕**，不是圆点——标签与画面不符。圆点应当属于状态机的字段，而不是全局可变状态。

### P1-4 按下→拖出→再拖入会重播一次放大弹跳

- `TrafficLightButton::update`（`traffic_lights.rs:759-769`）在“按住且重新进入”时再次 publish
  `on_press_start` → demo 再次 `retarget(1.18)`。`retarget` 保留速度，所以不会跳变，但会**重新给一次
  上升冲量**；配合 14 pt 的命中框（`TrafficLightButton::layout` 用 `self.width/height = size`，
  `traffic_lights.rs:703-719`）与 6 pt 的组级 slop（`bmol-designs::metrics::control_hover_slop`：
  ≤32 pt → 6.0，>32 pt → 16.0），按住时轻微手抖就会反复
  “取消 → 重入 → 再弹一次”，视觉上就是 hover/点击闪。

### P2 其他

- `ControlAction::Zoom` 与 `Expand` 在 demo 里都映射到 `expand_command()`（`:187`），语义重复。
- `close_disabled` 在 backend（`iced_backend.rs:1782` `_close_disabled`）与材质里**完全未使用**，
  只在 widget 里降级为 `is_active`。Disabled 样例的“不可用”语义没有真值。
- `TrafficLightsState` 的 `PartialEq`（`traffic_lights.rs:252-259`）用 `f32::EPSILON` 比较，
  且**忽略** `press_springs`，作为状态相等性判断不可靠。
- Native 组的红/黄按钮直连 `window::close` / `window::minimize`（`window_controls_demo.rs:184-189`，
  `liquid-glass-ui/src/window.rs:45-47`）：**点红=退出 demo，点黄=窗口消失且无法找回**
  （无 Dock/菜单的 frameless demo）。实验台里这属于误触代价过高。

---

## 2. 目标设计：一个状态机

### 2.1 库：把 `TrafficLightsState` 变成唯一的交互真值

```rust
// src/iced_ui/traffic_lights.rs

pub const PRESS_SCALE_REST: f32 = 1.0;   // 静止
pub const PRESS_SCALE_PEAK: f32 = 1.18;  // 按住时的可见峰值
// 删除 PRESS_SCALE_SETTLED（它把“过渡值”当成了“静止值”）
// press/release 都用 Spring::smooth_custom(~0.18, 0.0)（ζ≈1.0，无额外超调）
// 若要保留一点弹性，ζ≥0.9 等价于 bounce ≤0.1；用单测卡住 peak ≤ 1.20。

pub struct TrafficLightSlot {
    pub press: f32,                 // 0..1 tint
    pub scale: SpringMotion,        // 目标只有 REST / PEAK 两个取值
}

pub struct TrafficLightsState {
    pub group_hover: f32,
    pub group_hover_target: f32,     // 组级：三颗一起显字形（macOS 行为）
    pub slots: [TrafficLightSlot; 3],
    pub armed: Option<usize>,        // 指针按下的归属，跨帧存活
    pub focus: f32,                  // 窗口焦点 0..1（fade）
    pub edited: bool,                // 脏文档圆点
    pub expand_behavior: WindowExpandBehavior,
    last_tick: Option<Instant>,
}

pub enum TrafficLightsEvent {
    GroupHover(bool),
    PressStart(usize),
    PressCancel(usize),                       // 拖出：tint off + scale→REST
    PressEnd { index: usize, committed: bool },
    Action(WindowControlAction),
}
```

关键语义（这次要写进文档注释并加测试）：

| 事件 | tint | scale target | 备注 |
|------|------|--------------|------|
| `PressStart(i)` | 1 | PEAK | `armed = Some(i)` |
| `PressCancel(i)` | 0 | **REST** | 拖出即回弹（修 P0-2 / P1-4） |
| `PressStart(i)` 再次（重入） | 1 | PEAK | `retarget` 保留速度，**不重播弹跳** |
| `PressEnd{committed:true}` | 0 | REST | 发 `Action` |
| `PressEnd{committed:false}` | 0 | REST | 不发 `Action` |
| 任意 `PressEnd` | — | — | `armed = None`，**无条件**清状态（防卡死） |

`step(now)`：**一个 dt 走全部**（hover、tint、三个弹簧），沿用现有的真实 dt + `min(0.1)` 钳制；
`is_animating()` 仍是订阅开关的唯一依据。

### 2.2 库：把 `interactive` 拆成两个概念

```rust
pub enum ControlMode { Active, Inactive, EditedDot, Disabled }

pub struct ControlGroupConfig {
    pub size: f32, pub gap: f32, pub is_dark: bool,
    pub glyphs: bool,
    pub behavior: WindowExpandBehavior,
    pub mode: ControlMode,
    pub animates: bool,   // 是否挂 press 生命周期（所有样例都为 true）
}
```

`control_group(...)` 只负责把 state 渲染成元素；**是否执行窗口命令完全由调用方的 `Action` 分支决定**
（demo 里用 `executes: bool`）。这样 5 组样例都能有按下动画（修 P0-3），而命令执行仍是 Native 组专属。

### 2.3 demo：删掉并行状态，直接用库状态机

```rust
struct State {
    window: IcedWindowController,
    window_policy: IcedWindowPolicy,
    scheme: UiColorScheme,
    focused: bool,                                  // 新增：真实焦点
    lights: [TrafficLightsState; 5],                // 一组一个，索引 = ControlGroup::index()
    last_action: String,
    tuning: WindowControlTuning,
}
```

- `Message` 收敛为：`Lights { group, event: TrafficLightsEvent }` / `AnimationTick(Instant)` /
  `WindowFocused(bool)` / `WindowEvent(..)` / tuning / scheme / copy。
- `AnimationTick(Instant)`：`for s in &mut state.lights { s.step(now) }` → 推 GPU。
- 删除 `hover_targets/hover_progress/press_targets/press_progress/press_springs` 与
  `window_control_slot_index` / `control_group_scales` 两套索引算术（修 P1-1、P1-2）。
- 删除 demo 自建的 `control_group` / `finish_control_press*` 包装（`:808-844`、`:787-806`）。

### 2.4 backend：合成器变哑巴

`examples/playground/iced_backend.rs`：

- 删除 `glass_interaction` 里对 hover/press/focus 的自行平滑与覆盖（`:405-467`），
  只保留读取：hover / press / scale 由状态机单向下发。
- 合并写入口为 `set_window_control_interaction(id, hover, press, scale)`（或保留三个 setter 但
  **只留一个写入方**），删掉 `set_window_control_group_hover(bool)`（`window_demo` 一并切换）。
- `push_traffic_light_group` 的 `inactive` / `close_disabled` 改为从 state 传入的
  `ControlMode` 派生，让 Disabled/Edited 样例真正有意义（修 P2 的 `close_disabled` 死参数）。

---

## 3. 分阶段执行计划

> 每个阶段都能独立编译 + 独立验收；建议按顺序做，阶段 1 是其余阶段的地基。

### 阶段 1 — 修正 scale 契约（最小、最可见）

1. `traffic_lights.rs`：新增 `PRESS_SCALE_REST` / `PRESS_SCALE_PEAK`，删除 `PRESS_SCALE_SETTLED`；
   更新 L6 模块文档与 L67-73 注释。
2. `on_press_start/end/cancel` 统一走 `REST`/`PEAK` 两个目标；`on_press_cancel` 也 `retarget(REST)`。
3. 弹簧参数换成 `Spring::smooth_custom(0.18, 0.0)`（或 `bouncy_custom(0.18, 0.0)` 若想留一点点弹性，
   但必须 ζ≥0.9）。
4. demo `finish_control_press` 不再引用 `PRESS_SCALE_SETTLED`。
5. `window_demo.rs:409-419` 删掉手写的 `set_window_control_scale(id, 1.0)`（修 CPU/GPU 不同步）。

**验收**：新增单测（不依赖 GUI）
`press_then_release_returns_to_rest`：以 1/60 步进，断言
`peak ≤ 1.20`、`|rest - 1.0| < 1e-3`、release 段 `min ≥ 0.995`（无下冲）、`settle ≤ 0.35 s`。

### 阶段 2 — 事件语义与 widget 生命周期

1. `TrafficLightsEvent::PressEnd { index, committed }`；
   `TrafficLightButton` 只发“事实”（按下 / 离开 / 释放 / 释放时是否在框内），
   `committed` 由 widget 计算，`Action` 由状态机在 `PressEnd{committed:true}` 时返回。
2. `armed` 进状态机；**任何** `PressEnd` 都清 `armed`（即使 widget state 因 tree 重建丢失，
   下一次 release 也能收敛）。补一条 `CursorMoved`+`ButtonReleased` 的兜底：指针不在任何组内且
   有 `armed` → 视作 cancel。
3. `TrafficLightButton` 记录 `pressed_outside` 布尔而不是靠 `was_hovered/was_pressed` 组合，
   避免 `hovered` 只在事件到达时更新带来的陈旧判断（`traffic_lights.rs:753-769`）。
4. 命中框：把每颗按钮的 layout 扩到 `size + 2*slop`（视觉仍居中在 `size` 内），
   让“按住时手抖出框”不再触发 cancel；组级 slop 继续负责 glyph 揭示。

**验收**：纯函数化后单测事件序列：
`Press(0) → Cancel(0) → Press(0) → End{committed:true}` 断言 scale 轨迹**只有一次**上升、
tint 不闪断、结束回到 REST；`Press(0) → End{committed:false}` 不产生 `Action`。

### 阶段 3 — 单一状态机落地到 demo（最大改动）

1. `State.lights: [TrafficLightsState; 5]`，删掉全部并行数组与索引算术。
2. `subscription` 只看 `lights.iter().any(is_animating)`；tick 传 `Instant::now()`。
3. `control_group` 走新的 `ControlGroupConfig`（`animates: true` 全开，`executes` 仅 Native）。
4. `is_pressed` 不再硬编码 `false`（`traffic_lights.rs:1228/1256/1284`），改由 state 的
   `slots[i].press` 传入（修 P0-4）。
5. `ControlAction::Zoom` 与 `Expand` 二者选一，另一处标 `#[deprecated]` 或合并。

**验收**：5 组样例 hover 都显字形、按下都有 tint+放大、松手都回到 14/64 pt 原尺寸；
Native 组点击仍执行窗口命令，其余仅更新文字。

### 阶段 4 — 焦点 / 脏文档 / 模式

1. `subscribe iced::window::Event::Focused/Unfocused` → `Message::WindowFocused(bool)` →
   `state.focused`；Native 组由它驱动，Inactive 组固定 `focus = 0`。
2. 删除 `DOCUMENT_EDITED` global，改为 `TrafficLightsState.edited`；demo 给 Disabled 样例
   （标签 “Running · close status dot”）显式置 `ControlMode::EditedDot`。
3. `ControlMode` 下沉到 backend，`close_disabled` 死参数要么实现要么删除。

**验收**：切换窗口焦点，红绿灯在 active/inactive 之间平滑过渡；圆点样例真的显示圆点。

### 阶段 5 — backend 降耦 + 收尾

1. 删除 `glass_interaction` 的重复平滑与覆盖（`iced_backend.rs:402-467`），
   删除 `set_window_control_group_hover`。
2. `window_demo.rs` 同步切换到新的状态机 API（它是库的参考实现，必须一起改，否则又分叉）。
3. 清理 `TrafficLightsState::PartialEq`（要么比较 slot press + scale + armed，要么去掉 `PartialEq`）。
4. 修正 demo 的运行说明与 `--bin` 命名（`cargo run --example window_controls_demo`）。

### 阶段 6 — 误触保护（可选，建议）

Native 组加 “Preview mode” 开关（默认开）：开启时红了不发 `WindowCommand::Close`，只写 `last_action`；
否则点红即退出、点黄即窗口消失，实验台体验很差。

---

## 4. 验证方式

1. `cargo test -p bmol-window-shell` —— 状态机单测（阶段 1/2）+ 现有度量测试必须全绿。
2. `cargo clippy -p bmol-window-shell --all-targets` —— 无新增 warning。
3. 人工脚本（三个必查动作）：
   - 逐个点 5 组 × 3 颗：每颗都应 tint + 放大，松手回到原始直径（用 “1:1 reference · 14 pt” 组目视比对）。
   - 按住不松手拖出按钮 → 颜色熄灭、尺寸回弹；拖回 → 颜色回来、**不重播弹跳**；在框外松手 → 不触发命令。
   - 连点 10 次红/黄/绿：尺寸不得累积变大。
4. `window_demo`（参考实现）回归：红绿灯行为必须与 `window_controls_demo` 完全一致。
5. 记录一次数值回归：新增一个 `#[test]` 打印/断言弹簧峰值与稳定时间，防止以后又把目标值当峰值。

---

## 5. 需要你拍板的决策点

1. **拖出按钮时的语义**：按 AppKit 做“取消 → 尺寸回弹到 1.0”（本 plan 推荐），
   还是保留 commit `750ab52` 的“拖出后尺寸仍撑着直到松手”？
2. **峰值弹性**：peak 严格 1.18（临界阻尼，无弹跳）还是 1.18 略带回弹（ζ≈0.9，
   峰值 ~1.183）？
3. **Native 组是否直连窗口命令**：保留（点红退出 demo）还是加 Preview 开关？
4. **`TrafficLightsState` 的 API 兼容性**：是否允许破坏性重命名（`press_springs` →
   `slots[*].scale`、`PRESS_SCALE_SETTLED` 删除）？workspace 内 `window_demo` / `controller` /
   `scaffold` 会一起改，但对外是 semver break。

---

## 第五轮：按下改为整体变亮 + 跨仓库发布

**效果改动（liquid-rs，已推送）**

红绿灯的按下原本是指针局部的激励：`interactionLight` 为红绿灯选取 `u_press`，compose 阶段最多混入 12% 白并以 `exp(-dist)` 从光标衰减 —— 读起来是"有个高光在动"，而不是"控件被按下了"。

现在：
- `interactionLight` 对红绿灯返回 0，彻底不施加指针局部亮斑（顺带消除组级 hover 把同组三颗都点亮的隐患）。
- 在 **body response 之后**加一次空间均匀的整体增益：
  `outColor * (1 + 0.12 * press) + tint * 0.06 * press`。
  放在 body response 之后是关键：球体下半部分是 transmission 混合（最多 22% 混向背景）、边缘带 curvatureShadow 吸收（最高 0.92）；放在前面只有上半球和中间会变亮，又变成局部效果。
- 仍然遵守 `FEATURE_REDUCED_MOTION`。

**发布链**

| 仓库 | commit | 内容 |
|---|---|---|
| liquid-rs | `f5650a3` (main) | shader 整体按下增益；render 测试里补注释说明 `size_of::<GlassUniform>()` 守卫 |
| bmol-iced | `3ab7be0` (main) | liquid-rs 依赖从 `tag = "v0.1.3"` 移到 `branch = "main"`（只改依赖行，其余 WIP 原样保留） |
| bmol-window-shell | 未提交 | liquid-rs 直连依赖移到 `branch = "main"` |

**验证**：`liquid-glass-render` 12 项测试通过（含 `create_shader_module`，即 WGSL 过了 naga 校验）；shell `cargo test --workspace` 65 项通过；`cargo build --examples` 通过。`cargo tree` 确认 `bmol-iced(branch=main#3ab7be0e) → liquid-glass-render(branch=main#f5650a3)`，即 demo 渲染用的就是带修复的 shader。

**仍存在的第二份依赖**：published `bmol-window-shell v0.1.8` tag 里带着旧的 `bmol-window-glass`，它把 `liquid-glass-scene` / `liquid-glass-render` 又拉了一份 `tag=v0.1.3`。这份是 bmol-iced 的 `bmol-window-shell` 依赖造成的循环，本地代码并不使用它，因此不影响渲染结果与类型对齐；彻底清掉需要：提交本仓库的红绿灯重构 → 打新 tag → 把 bmol-iced 的 `bmol-window-shell` 依赖前移。

**待确认**：`0.12 / 0.06` 这两个增益未经目视校准。另外"非聚集情况中间光感仅为 55%"我未能对应到唯一位置，候选有三个：`pressLight` 的 `0.62 + 0.38*localFocus`（但对红绿灯是死代码）、`trafficLightBodyResponse` 的中间光 `0.045 + 0.055`、以及 `EXTERNAL_TAIL_STRENGTH = 0.55`（与中间光无关）。若指第二项，需要单独上调。

---

## 第六轮：bead 变体的两个结构缺陷 + dpr

前几轮之所以一直修不好 bead，是因为在**猜**，而且猜错了方向。这一轮改为读源码定论。

### 6.1 bead 的圆其实一开始就是对的

`mainSDF(p1, p2, p)` 里的 `u_mouseSpring` 并不是"指针"，而是 **node 中心**：

```
uniform_for_node:  mouse_and_spring: [pointer_x, pointer_y, center_x, center_y]
center_x = bounds.x + bounds.width * 0.5
center_y = size.height - bounds.y - bounds.height * 0.5      // y-up 物理像素
```

所以 `p2n = p2 + pixel/res.y = (pixel - center)/res.y`，`d2` 就是**单颗控件自己的圆**
（`GlassShape::Circle` → `roundness 2.0` + `radius = min(w,h)*0.5`，经 `roundedRectSDF`
退化为正圆）。`beadOffset = (0 - pixel)/res.y - p2 = (center - pixel)/res.y = -p2n`，
长度相同；`nx` 的符号与 B 相反但所有用到 `nx` 的项都是偶函数，`ny` 的符号与 B 一致
（都表示"在中心下方"）。**算术上从来没有错。**

之前"品红铺满整组矩形"的观测，正确的解释是下面 6.2，而不是"圆算错了"。

### 6.2 两个真正的缺陷

1. **整块正方形**：glass pass 是 `blend: None`，片元**直接替换** render target。
   bead 分支原先 `return vec4f(bead, shapeAlpha * opacity)`，而 `bead` 在圆外等于
   `u_tint.rgb` —— alpha 拦不住 RGB，于是整个 node quad 被涂成 tint 色。quad =
   node bounds = 随按压弹簧缩放的球体 bounds，所以这个正方形**还会跟着动画缩放**。
   物理路径早在 1486 行就为同一问题留了注释（"leaves a visible red box"），解法是
   `sampleActualBackdrop` 后在 shader 内自己合成。bead 需要遵守同一约定。

2. **覆盖来自合并剪影**：原来用 `shapeAlpha`（来自 `merged`），而 `merged` 里含
   `shape 1` 和全部 fused layer。改为把 node 自身剪影从 `mainSDF` 拆成 `shapeSDF`，
   bead 只从 `shapeSDF` 取覆盖。

### 6.3 顺带修掉的 backdrop pin（native 路径）

`scale_scene` 末尾无条件执行 `node.backdrop.bounds = node.visual_bounds()`，
把 `push_traffic_light_group` 设的"静止尺寸"pin 覆盖掉了，于是 native 路径上
**模糊矩形又跟着按压弹簧一起长大**。改为按 pre-scale 几何判断该 region 是否跟随形状：
跟随则重取 `visual_bounds()`，被 pin 过则原地乘缩放因子。

### 6.4 dpr：边圈窄了 29%

B 的光栅器把 rim span 写成 `2 * max(1.8, 2.4*sqrt(size/14))`，`size` 是**逻辑点**、
前面的 2 是固定 dpr。shader 里 `u_dpr` 恒为 1.0（因为所有 shape uniform 已经是物理像素，
`u_dpr` 只是历史遗留的 no-op），两个项都塌回 1x：

| 控件 | B | 改前 shader | 改后 shader |
|---|---|---|---|
| 64 pt | `2*max(1.8, 2.4*√(64/14))` = 10.26 px | 7.26 px | 10.26 px |
| 14 pt | `2*max(1.8, 2.4*1)` = 4.80 px | 3.39 px | 4.80 px |

两个项的偏差倍数还不一样（2x 与 √2），所以不是一个能凑的系数。做法：
`GlassRenderOptions::scale_factor`（默认 1.0）→ `resolution_dpr_pad` 的第 4 个空槽 →
WGSL `u_renderScale`，**只给 bead 用**。守卫测试钉死"任何 shape 槽位都不得跟随它"。

### 6.5 验证

| 项目 | 结果 |
|---|---|
| `liquid-rs` `cargo test --workspace` | 25 项通过（含 `create_shader_module` 的 naga 校验、`GlassUniform` = 528 布局守卫） |
| shell `cargo test --workspace` | 69 项通过（含新增 `scaling_the_scene_keeps_a_pinned_backdrop_pinned`） |
| physical 变体截图 | dpr 改动**前后 bit-identical**（sha256 `b3ad1e3e…`），证明只影响 bead |
| bead 变体截图 | 正方形消失；圆盘正确；深红暗边对称出现在左右两侧 |
| 像素级核对 | 圆盘中线 x=36..140 恒为 `(255,94,84)`（无横向不对称）；纵向 G 由顶部 85 升到底部 101，即 B 的 `axial` 垂直辉光 |

**仍待目视校准**：bead 的 `0.88` 基数在 B 里是 **sRGB 空间**乘法，而 shader 在
**线性光**里做，两者不完全等价（约 4% 偏亮），且红色内部 R 已顶到 255。这一项是否能接受，
需要你看过 bead 之后判断；相关旋钮（center glow / saturation lift / rim span / core span /
edge darkness / highlight）都在面板上。

### 6.6 本轮提交

| 仓库 | commit | 内容 |
|---|---|---|
| liquid-rs | `e3fd302` | bead 合成到 backdrop 之上；`shapeSDF` 拆出 |
| liquid-rs | `d23ff66` | `GlassRenderOptions::scale_factor` + `u_renderScale` |
| bmol-window-shell | `5934a86` | backdrop pin 跨缩放保持；demo 支持 `LIQUID_GLASS_TRAFFIC_LIGHT_MATERIAL=bead\|physical` |
| bmol-window-shell | `a886ea9` | 把 `viewport.scale_factor()` 送进 render options |

### 6.7 sRGB 重算（已按你的确认执行）

B 的光栅器把所有常量都作用在 **sRGB 编码值**上，然后直接写出 sRGB 字节：

```
base_r = button_r * (0.88 + depth_lift)          // sRGB 空间
final_r = (base_r - dark_drop + light_contrib).clamp(0.0, 1.0)
raw[idx] = (final_r * 255.0).round()             // 写成 sRGB 字节
```

而 shader 在**线性光**里算（`u_tint.rgb` 经 `srgb_to_linear_rgba` 解码，输出到 sRGB target
由硬件编码）。同一组常量在两个空间里差别很大：

| 项 | sRGB 空间 | 线性空间后经编码 |
|---|---|---|
| `0.88` 基数 | −12% | 约 −5.7% |
| 增益 `×(0.88+lift)` | 直接乘 | 被 sRGB 曲线压缩 |
| `52/255` 暗边减法 | 直接减 | 感知上弱得多 |

**做法**：在 bead 分支内 `encodeSrgb(u_tint.rgb)` 进入 B 的空间，算完再 `decodeSrgb`
回到线性交给 target。`u_interactionResponse.z` 的按下加色也从"线性 tint"改为"sRGB tint"，
消掉那一项自身的空间错配。新增 `encodeSrgbChannel` / `decodeSrgbChannel` / `encodeSrgb` /
`decodeSrgb` 四个 helper（含 0.0031308 / 0.04045 的分段线性段）。

**数值验证**（64pt 红球，dpr 2，crop 中心 (87.5, 87)，半径 63.5px）：

| | 改前 | 改后 | B 公式手算 |
|---|---|---|---|
| 盘心 | (254, 95, 83) | (254, 105, 92) | (255, 108, 102) |
| 边缘暗边最深 G | 203 | 157 | — |

中心与手算的残差来自 demo 实际 tuning 与 B 的 `PhysicalTrafficLightTuning::default()`
不同，不是公式误差。

**对称性**：`|dx| ≤ 48` 范围内左右完全相同（Δmax ≤ 2）；只有最外 1–2px 有 7–17 的差异，
因为圆心落在半像素 x=87.5，左右边缘的亚像素相位不同。**不存在横向不对称**——之前肉眼
看到的"左暗右亮"是 WebP 预览的边缘混叠错觉。

**暗边系数反推**：边缘 `colour_srgb = tint_srgb×0.88 − (52/255)×|nx|²×I`，代入实测解出
`I ≈ 2.1`，与 B 的 `dark_rim_intensity = 2.00` 相符，残差 ~0.04 为 AA 混合。

**验证**：physical 变体截图在 sRGB 改动前后**依然 bit-identical**（`b3ad1e3e…`），
证明 helper 与改动只作用于 bead。

**未验证**：暗色模式（`mode_dark = 1`，走 `bright` 亮边而非 `darkDrop` 暗边）。它用的是
同一套 sRGB 公式，但需要切到 dark scheme 才能截图，本轮未做。

**提交**：liquid-rs `0c28e8b`。

### 6.8 误提交的 liquid-rs WIP

`2cc81a3`（我误将你的 content-glass WIP 用错误消息提交并推送）—— 按你的决定**保留现状**，
不改写历史。
