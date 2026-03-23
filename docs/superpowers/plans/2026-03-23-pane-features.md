# Plot Viewer Pane Features Implementation Plan

> **Status:** Archived as mostly shipped. The key pane interactions in this document — zoom reset, solo mode, X-window switching, cursor movement, filter mode, and dynamic chart messaging — are already present in the current TUI.
>
> **Roadmap note:** Keep this file only as a historical checklist. Any future pane work should start from current behavior in `rust/plot-viewer/src/app.rs`, `src/input.rs`, and `src/render/chart.rs`, not from unchecked boxes below.

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add pane-specific interactive features so Tab-switching between Accounts and Plot panes is purposeful.

**Architecture:** Features split into render-side state (ChartState fields in render/mod.rs, visual logic in chart.rs) and app-side state (App struct, handle_action routing in app.rs). Chunk 1 defines interfaces; Chunk 2 wires them together; Chunk 3 polishes UX.

**Tech Stack:** Rust, Ratatui, Crossterm

---

## Shipped outcome summary

- `InputAction` already includes `ResetZoom`, `ToggleSolo`, `XWindow(u8)`, and `FilterEnter`.
- `ChartState` already carries `x_lower`, `solo`, and `cursor_x`.
- The chart already uses dynamic X bounds, solo filtering, cursor overlay, and dynamic footer/help text.
- Remaining work, if any, should come from real-account usage observations rather than from replaying this plan verbatim.

---

## 檔案異動

| 檔案 | 動作 |
|------|------|
| `rust/plot-viewer/src/input.rs` | 加 `ResetZoom`, `ToggleSolo`, `XWindow(u8)`, `FilterEnter` |
| `rust/plot-viewer/src/render/mod.rs` | `ChartState` 加 `x_lower`, `solo`, `cursor_x` |
| `rust/plot-viewer/src/render/chart.rs` | 動態 X bounds、solo 過濾、cursor 線、動態 X labels、動態 axis 說明列 |
| `rust/plot-viewer/src/app.rs` | 新 App 欄位、handle_action 路由、filter render、context footer、auto-reload、`format_duration_short`、`auto_y_lower` |
| `rust/plot-viewer/tests/rust_cli_migration.rs` | 確認無編譯錯誤 |

---

## Navigation 設計（最終狀態）

| Pane | ↑↓ | ←→ | Enter/n/d/u/a | s | 1/3/7 | +/-/r | / |
|------|----|-----|--------------|---|-------|-------|---|
| Accounts | 導航 profiles | — | 有效 | — | — | zoom | 進入 filter |
| Plot | 導航 profiles | 移動游標 | **無效** | solo toggle | X 視窗 | zoom | — |

---

## Chunk 1: Interface definitions — input.rs + render (Lane A / Lane B 可並行)

### Task 1 (Lane A): input.rs — 新增所有 InputAction variants

**Files:**
- Modify: `rust/plot-viewer/src/input.rs`

目前 input.rs 有：`Quit, Up, Down, Left, Right, Enter, Backspace, NextFocus, PreviousFocus, ZoomIn, ZoomOut, RefreshSelected, RefreshAll, Rename, Delete, Character(char), Cancel`

- [ ] **Step 1: 加入新 variants**

```rust
// 在 PreviousFocus 之後加入：
ResetZoom,          // r
ToggleSolo,         // s (only meaningful in Plot pane)
XWindow(u8),        // 1 → 1d, 3 → 3d, 7 → 7d
FilterEnter,        // / (enter filter mode in Accounts pane)
```

- [ ] **Step 2: 加入 key bindings**

```rust
// 在 map_event match 裡加（放在 ZoomOut 之後）：
KeyCode::Char('r') => Some(InputAction::ResetZoom),
KeyCode::Char('s') => Some(InputAction::ToggleSolo),
KeyCode::Char('1') => Some(InputAction::XWindow(1)),
KeyCode::Char('3') => Some(InputAction::XWindow(3)),
KeyCode::Char('7') => Some(InputAction::XWindow(7)),
KeyCode::Char('/') => Some(InputAction::FilterEnter),
```

**注意：** `'/'` 原本會 fall through 到 `Character('/')` — 現在改為 `FilterEnter`，所以要放在 `Char(ch) => Character` 之前。

- [ ] **Step 3: 確認編譯**

```bash
cd rust/plot-viewer && cargo build 2>&1
```
Expected: 只有 dead_code warnings（新 variants 尚未使用），無 error。

- [ ] **Step 4: Commit**

```bash
git add rust/plot-viewer/src/input.rs
git commit -m "feat(input): add ResetZoom, ToggleSolo, XWindow, FilterEnter actions"
```

---

### Task 2 (Lane B): render/mod.rs + render/chart.rs

**Files:**
- Modify: `rust/plot-viewer/src/render/mod.rs`
- Modify: `rust/plot-viewer/src/render/chart.rs`

#### Step 1: render/mod.rs — 擴充 ChartState

- [ ] 在 `ChartState` struct 加三個欄位：

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ChartState<'a> {
    pub series: Vec<ChartSeries<'a>>,
    pub seven_day_points: Vec<ChartPoint>,
    pub five_hour_band: FiveHourBandState<'a>,
    pub five_hour_subframe: FiveHourSubframeState<'a>,
    pub total_points: usize,
    pub y_lower: f64,
    pub y_upper: f64,
    // --- new ---
    pub x_lower: f64,       // 0.0 (7d), 4.0 (3d), 6.0 (1d) — x軸左界
    pub solo: bool,          // 只顯示 selected series
    pub cursor_x: Option<f64>, // None = 無游標; Some(x) = 游標位置 0..7
}
```

#### Step 2: render/chart.rs — 動態 X bounds + solo filter + cursor line

- [ ] 在 `render_usage_chart` 開頭加 x_bounds 局部變數（取代常數）：

```rust
let x_bounds = [chart_state.x_lower, 7.0_f64];
let y_bounds = [chart_state.y_lower, chart_state.y_upper];
```

- [ ] 移除 `const X_AXIS_BOUNDS`（已替換為局部 `x_bounds`）。在原本所有使用 `X_AXIS_BOUNDS` 的地方改用 `x_bounds`。

- [ ] 加 solo filter — 在計算 `series_points` 之前，先建立 visible_series：

```rust
let visible_series: Vec<&ChartSeries<'_>> = if chart_state.solo {
    chart_state.series.iter().filter(|s| s.style.is_selected).collect()
} else {
    chart_state.series.iter().collect()
};
```

然後 `series_points` 和 datasets 都改用 `visible_series` 而非 `chart_state.series.iter()`：

```rust
let series_points = visible_series
    .iter()
    .map(|series| {
        series
            .points
            .iter()
            .map(|point| (point.x, point.y.clamp(y_bounds[0], y_bounds[1])))
            .collect::<Vec<_>>()
    })
    .collect::<Vec<_>>();

// subframe_per_series 同樣改用 visible_series：
let subframe_per_series: Vec<(Color, Vec<(f64, f64)>, Vec<(f64, f64)>)> = visible_series
    .iter()
    .filter_map(|series| { /* 同原本邏輯 */ })
    .collect();

// datasets 也改用 visible_series.iter().zip(series_points.iter())：
let mut datasets = visible_series
    .iter()
    .zip(series_points.iter())
    .map(|(series, points)| { /* 同原本 */ })
    .collect::<Vec<_>>();
```

- [ ] 加 cursor 垂直線 dataset（在 subframe datasets push 之後）：

```rust
// cursor line — 在最後 push，確保顯示在最上層
let cursor_points: Option<Vec<(f64, f64)>> = chart_state.cursor_x.map(|cx| {
    vec![(cx, y_bounds[0]), (cx, y_bounds[1])]
});
if let Some(ref pts) = cursor_points {
    datasets.push(
        Dataset::default()
            .name("")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::DarkGray))
            .data(pts),
    );
}
```

- [ ] 動態 X axis labels（取代原本 hardcode 的 `["0d", "3.5d", "7d"]`）：

```rust
let x_mid = (x_bounds[0] + 7.0) / 2.0;
let x_label_lo = format!("{:.1}d", x_bounds[0]);
let x_label_mid = format!("{:.1}d", x_mid);
let x_label_hi = "now".to_string();
```

然後 `.labels([x_label_lo.as_str(), x_label_mid.as_str(), x_label_hi.as_str()])`

- [ ] 更新 `format_five_hour_band_line` 保持不動（已有 reason fallback）。

- [ ] band_summary 第三行改為動態（游標位置 or 軸說明）：

在 `render_chart` 裡把：
```rust
Line::from("Axis: 7d window, Y = usage%"),
```
改為：
```rust
Line::from(match chart_state.cursor_x {
    Some(cx) => format!("Cursor: {:.1}d ago  (←→ move, ↑↓ profile)", 7.0 - cx),
    None => {
        let window_label = match (chart_state.x_lower * 10.0).round() as i32 {
            0 => "7d",
            40 => "3d",
            60 => "1d",
            _ => "?d",
        };
        format!("Window: {} · +/-=zoom · r=reset · 1/3/7=window", window_label)
    }
}),
```

#### Step 3: 修正 chart.rs 測試的 ChartState fixture

目前測試 `ChartState` 需要加 `x_lower: 0.0, solo: false, cursor_x: None`：

```rust
// 兩個測試的 chart: ChartState { ... } 都加上：
x_lower: 0.0,
solo: false,
cursor_x: None,
```

- [ ] **Step 4: 確認測試通過**

```bash
cd rust/plot-viewer && cargo test 2>&1
```
Expected: 全部通過，0 error。

- [ ] **Step 5: Commit**

```bash
git add rust/plot-viewer/src/render/mod.rs rust/plot-viewer/src/render/chart.rs
git commit -m "feat(render): dynamic X/Y bounds, solo filter, cursor line, axis labels"
```

---

## Chunk 2: App state + behavior (depends on Chunk 1)

### Task 3: App struct — 新增欄位 + 初始化

**Files:**
- Modify: `rust/plot-viewer/src/app.rs`

- [ ] **Step 1: 在 `App` struct 加新欄位**

```rust
pub struct App {
    // ... 現有欄位 ...
    // --- new ---
    solo_mode: bool,
    x_window_days: u8,           // 1, 3, 或 7
    cursor_x: Option<f64>,
    filter_input: Option<String>, // None = 不過濾；Some(s) = filter mode
    last_auto_reload: std::time::Instant,
}
```

- [ ] **Step 2: `App::load()` 初始化新欄位**

```rust
Ok(Self {
    // ... 現有 ...
    solo_mode: false,
    x_window_days: 7,
    cursor_x: None,
    filter_input: None,
    last_auto_reload: std::time::Instant::now(),
})
```

- [ ] **Step 3: `App::from_profile_names()` 同樣初始化**

```rust
Self {
    // ... 現有 ...
    solo_mode: false,
    x_window_days: 7,
    cursor_x: None,
    filter_input: None,
    last_auto_reload: std::time::Instant::now(),
}
```

- [ ] **Step 4: 加 `use std::time::Instant;`**（若 `std::time` 已 import，只需加 `Instant` 到 use list）

目前頂部有 `use std::time::{Duration, SystemTime, UNIX_EPOCH};`，改為：
```rust
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
```

- [ ] **Step 5: 確認編譯**

```bash
cd rust/plot-viewer && cargo build 2>&1
```
Expected: 可能有 unused field warnings，無 error。

---

### Task 4: handle_action 路由重構

**Files:**
- Modify: `rust/plot-viewer/src/app.rs`

#### Accounts pane gate

- [ ] **Step 1: Enter/Rename/Delete/RefreshSelected 加 pane guard**

```rust
InputAction::Enter => {
    if self.pane_focus == PaneFocus::Accounts {
        self.activate_selected_profile()?;
    }
}
InputAction::Rename => {
    if self.pane_focus == PaneFocus::Accounts {
        self.open_rename_dialog();
    }
}
InputAction::Delete => {
    if self.pane_focus == PaneFocus::Accounts {
        self.open_delete_dialog();
    }
}
InputAction::RefreshSelected => {
    if self.pane_focus == PaneFocus::Accounts {
        self.refresh_selected_profile(true)?;
    }
}
```

注意：`RefreshAll`（a 鍵）維持兩個 pane 都有效。

#### Plot pane navigation — ←→ 改為游標，↑↓ 在 Plot pane 也可用

- [ ] **Step 2: 修改 Left/Right 行為**

```rust
InputAction::Left => {
    if self.pane_focus == PaneFocus::Plot {
        self.move_cursor(-1);
    }
}
InputAction::Right => {
    if self.pane_focus == PaneFocus::Plot {
        self.move_cursor(1);
    }
}
```

- [ ] **Step 3: 加 `move_cursor` helper method**

```rust
fn move_cursor(&mut self, direction: isize) {
    let x_lower = 7.0 - self.x_window_days as f64;
    let step = self.x_window_days as f64 / 20.0;
    let current = self.cursor_x.unwrap_or(7.0);
    let next = (current + direction as f64 * step).clamp(x_lower, 7.0);
    self.cursor_x = Some(next);
}
```

（↑↓ 不需要改，`step_profile` 現在在兩個 pane 都響應 Up/Down，與新設計一致。）

#### 新 actions routing

- [ ] **Step 4: 加新 actions 到 match**

```rust
InputAction::ResetZoom => {
    self.y_zoom_lower = auto_y_lower(&self.profiles);
}
InputAction::ToggleSolo => {
    if self.pane_focus == PaneFocus::Plot {
        self.solo_mode = !self.solo_mode;
    }
}
InputAction::XWindow(days) => {
    if self.pane_focus == PaneFocus::Plot {
        self.x_window_days = days;
        // 游標若超出新視窗範圍，clamp
        if let Some(cx) = self.cursor_x {
            let new_lower = 7.0 - days as f64;
            self.cursor_x = Some(cx.clamp(new_lower, 7.0));
        }
    }
}
InputAction::FilterEnter => {
    if self.pane_focus == PaneFocus::Accounts {
        self.filter_input = Some(String::new());
    }
}
```

#### Filter mode input 攔截

目前 `handle_action` 開頭只檢查 `dialog.is_some()`。加 filter mode 檢查：

- [ ] **Step 5: 在 dialog check 之後加 filter mode check**

```rust
fn handle_action(&mut self, action: InputAction) -> Result<()> {
    if self.dialog.is_some() {
        return self.handle_dialog_action(action);
    }
    if self.filter_input.is_some() {
        return self.handle_filter_action(action);
    }
    // ... existing match ...
}
```

- [ ] **Step 6: 加 `handle_filter_action` method**

```rust
fn handle_filter_action(&mut self, action: InputAction) -> Result<()> {
    match action {
        InputAction::Quit | InputAction::Cancel => {
            self.filter_input = None;
        }
        InputAction::Enter => {
            // 確認 filter，離開 filter mode 但保留篩選顯示
            self.filter_input = None;
        }
        InputAction::Backspace => {
            if let Some(f) = self.filter_input.as_mut() {
                f.pop();
                if f.is_empty() {
                    self.filter_input = None;
                }
            }
        }
        InputAction::Character(ch) => {
            if let Some(f) = self.filter_input.as_mut() {
                f.push(ch);
            }
        }
        InputAction::Up => self.step_profile_filtered(-1),
        InputAction::Down => self.step_profile_filtered(1),
        _ => {}
    }
    Ok(())
}
```

- [ ] **Step 7: 加 `filtered_profile_indices` + `step_profile_filtered`**

```rust
fn filtered_profile_indices(&self) -> Vec<usize> {
    match &self.filter_input {
        None => (0..self.profiles.len()).collect(),
        Some(f) => {
            let lower = f.to_lowercase();
            self.profiles
                .iter()
                .enumerate()
                .filter(|(_, p)| p.profile_name.to_lowercase().contains(&lower))
                .map(|(i, _)| i)
                .collect()
        }
    }
}

fn step_profile_filtered(&mut self, delta: isize) {
    let indices = self.filtered_profile_indices();
    if indices.is_empty() {
        return;
    }
    // 找目前 selected 在 filtered list 中的位置
    let pos = indices
        .iter()
        .position(|&i| i == self.selected_profile_index)
        .unwrap_or(0);
    let len = indices.len() as isize;
    let next_pos = (pos as isize + delta).rem_euclid(len) as usize;
    self.selected_profile_index = indices[next_pos];
}
```

- [ ] **Step 8: 確認編譯**

```bash
cd rust/plot-viewer && cargo build 2>&1
```

---

### Task 5: run_loop — 30s auto-reload

**Files:**
- Modify: `rust/plot-viewer/src/app.rs`

- [ ] **Step 1: 在 run_loop 加定時 reload 檢查**

```rust
fn run_loop(&mut self, terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
    while !self.should_quit {
        terminal.draw(|frame| self.render(frame))?;

        if event::poll(Duration::from_millis(150))? {
            let event = event::read()?;
            if let Some(action) = input::map_event(&event) {
                self.handle_action(action)?;
            }
        }

        // Auto-reload every 30s to pick up cron-refreshed usage data
        if self.last_auto_reload.elapsed() >= Duration::from_secs(30) {
            let _ = self.reload_profiles(false, None);
            self.last_auto_reload = Instant::now();
        }
    }
    Ok(())
}
```

- [ ] **Step 2: 確認編譯**

```bash
cd rust/plot-viewer && cargo build 2>&1
```

---

### Task 6: render_left_pane + context footer + AppRenderState

**Files:**
- Modify: `rust/plot-viewer/src/app.rs`

#### render_left_pane — filter mode 顯示

- [ ] **Step 1: 過濾 profile list + filter indicator**

```rust
fn render_left_pane(&self, frame: &mut Frame, area: Rect) {
    let indices = self.filtered_profile_indices();
    let list_lines = (indices.len().max(3).min(10) + 2) as u16;

    let [list_area, detail_area] =
        Layout::vertical([Constraint::Length(list_lines), Constraint::Min(0)]).areas(area);

    let profiles_title = match (&self.filter_input, self.pane_focus == PaneFocus::Accounts) {
        (Some(f), _) => format!("Profiles [/{}]", f),
        (None, true) => "Profiles [active]".to_string(),
        (None, false) => "Profiles".to_string(),
    };

    let items = indices
        .iter()
        .map(|&i| {
            let profile = &self.profiles[i];
            let color = render::SERIES_COLORS[i % render::SERIES_COLORS.len()];
            let prefix = if profile.is_current { "▶ " } else { "  " };
            let unsaved_tag = if profile.saved_name.is_none() { " [unsaved]" } else { "" };
            let usage = format_usage_badge(&profile.usage_view);
            let label = format!("{}{}{unsaved_tag}", prefix, profile.profile_name);
            ListItem::new(Line::from(vec![
                Span::styled(label, Style::default().fg(color)),
                Span::raw(format!(" {usage}")),
            ]))
        })
        .collect::<Vec<_>>();

    let selected_pos_in_filtered = indices
        .iter()
        .position(|&i| i == self.selected_profile_index);

    let mut state = ListState::default();
    state.select(selected_pos_in_filtered);
    let list = List::new(items)
        .block(Block::default().title(profiles_title).borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, list_area, &mut state);

    let detail_lines =
        render_account_detail(self.selected_profile(), self.status_message.as_deref(), &self.cron_status);
    let details = Paragraph::new(Text::from(detail_lines))
        .block(Block::default().title("Details").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    frame.render_widget(details, detail_area);
}
```

#### Context-sensitive footer

- [ ] **Step 2: 改 render() 的 footer**

```rust
let footer_lines = match self.pane_focus {
    PaneFocus::Accounts => vec![
        Line::from("Enter=switch · n=rename · d=delete · u=refresh · a=all · q=quit"),
        Line::from(format!(
            "Tab=plot [{}] · ↑↓=navigate · /=filter{}",
            if self.pane_focus == PaneFocus::Plot { "active" } else { "" },
            if self.filter_input.is_some() { " (Esc=clear)" } else { "" }
        )),
    ],
    PaneFocus::Plot => vec![
        Line::from(format!(
            "1/3/7=window · s=solo{} · +/-=zoom · r=reset · a=refresh · q=quit",
            if self.solo_mode { "[ON]" } else { "" }
        )),
        Line::from("Tab=accounts · ↑↓=profile · ←→=cursor"),
    ],
};
let footer = Paragraph::new(Text::from(footer_lines)).wrap(Wrap { trim: true });
frame.render_widget(footer, footer_area);
```

#### AppRenderState — 傳新欄位

- [ ] **Step 3: 更新 `AppRenderState` struct**

```rust
pub(crate) struct AppRenderState<'a> {
    profiles: &'a [ProfileEntry],
    selected_profile_index: usize,
    y_zoom_lower: f64,
    // --- new ---
    solo: bool,
    x_window_days: u8,
    cursor_x: Option<f64>,
}
```

- [ ] **Step 4: 更新 `render()` 裡的 AppRenderState 構造**

```rust
let render_state = AppRenderState {
    profiles: &self.profiles,
    selected_profile_index: self.selected_profile_index,
    y_zoom_lower: self.y_zoom_lower,
    solo: self.solo_mode,
    x_window_days: self.x_window_days,
    cursor_x: self.cursor_x,
};
```

- [ ] **Step 5: 更新 `chart_state()` impl 傳入新欄位**

```rust
fn chart_state(&self) -> ChartState<'_> {
    let mut state = build_chart_state(self.profiles, self.selected_profile_index);
    state.y_lower = self.y_zoom_lower;
    state.y_upper = 100.0;
    state.x_lower = 7.0 - self.x_window_days as f64;
    state.solo = self.solo;
    state.cursor_x = self.cursor_x;
    state
}
```

- [ ] **Step 6: build_chart_state 的初始 ChartState 加新欄位預設值**

在 `build_chart_state` 裡的 `ChartState { ... }` literal 加：
```rust
x_lower: 0.0,
solo: false,
cursor_x: None,
```

- [ ] **Step 7: 確認測試通過**

```bash
cd rust/plot-viewer && cargo test 2>&1
```
Expected: 全部通過，0 error。

- [ ] **Step 8: Commit**

```bash
git add rust/plot-viewer/src/app.rs
git commit -m "feat(app): pane gate, filter mode, solo, cursor, X window, auto-reload, context footer"
```

---

## Chunk 3: Quality polish

### Task 7: format_duration_short — 人類可讀時間

**Files:**
- Modify: `rust/plot-viewer/src/app.rs`

`reset_after_seconds` 型別為 `i64`（負值視為 0）。

- [ ] **Step 1: 加 `format_duration_short` function**

放在 `format_age` 附近：

```rust
/// Format a duration in seconds as "3d 9h", "9h 38m", "45m", etc.
/// Uses the two largest non-zero units.
fn format_duration_short(secs: i64) -> String {
    let secs = secs.max(0) as u64;
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    if days > 0 {
        if hours > 0 {
            format!("{}d {}h", days, hours)
        } else {
            format!("{}d", days)
        }
    } else if hours > 0 {
        if mins > 0 {
            format!("{}h {}m", hours, mins)
        } else {
            format!("{}h", hours)
        }
    } else {
        format!("{}m", mins.max(1))
    }
}
```

- [ ] **Step 2: 在 `render_account_detail` 裡替換 `reset in {}s`**

```rust
// 改前：
"Weekly:  {:.0}% used, reset in {}s", w.used_percent, w.reset_after_seconds
// 改後：
"Weekly:  {:.0}% used, reset in {}", w.used_percent, format_duration_short(w.reset_after_seconds)

// 改前：
"5h:      {:.0}% used, reset in {}s", w.used_percent, w.reset_after_seconds
// 改後：
"5h:      {:.0}% used, reset in {}", w.used_percent, format_duration_short(w.reset_after_seconds)
```

- [ ] **Step 3: 加單元測試**

```rust
#[cfg(test)]
mod duration_tests {
    use super::*;

    #[test]
    fn format_duration_short_covers_all_tiers() {
        assert_eq!(format_duration_short(293927), "3d 9h");  // 293927 = 3*86400 + 9*3600 + 38*60 + 47
        assert_eq!(format_duration_short(86400),  "1d");
        assert_eq!(format_duration_short(9000),   "2h 30m");
        assert_eq!(format_duration_short(3600),   "1h");
        assert_eq!(format_duration_short(120),    "2m");
        assert_eq!(format_duration_short(30),     "1m");     // < 1m rounds up to 1m
        assert_eq!(format_duration_short(0),      "1m");
        assert_eq!(format_duration_short(-1),     "1m");     // negative clamped
    }
}
```

- [ ] **Step 4: 執行測試確認通過**

```bash
cd rust/plot-viewer && cargo test format_duration 2>&1
```

- [ ] **Step 5: Commit**

```bash
git add rust/plot-viewer/src/app.rs
git commit -m "feat(details): human-readable reset time (3d 9h style)"
```

---

### Task 8: Y-axis auto-fit on startup + reset

**Files:**
- Modify: `rust/plot-viewer/src/app.rs`

Y 軸預設：從 data 最小值（取 floor to 5%）到 100%，而非 0~100%。`r` 鍵 reset 也回到此值。

- [ ] **Step 1: 加 `auto_y_lower` function**

```rust
/// Compute the default y_zoom_lower from actual data: floor(min_y, 5%).
/// Returns 0.0 if no data.
fn auto_y_lower(profiles: &[ProfileEntry]) -> f64 {
    let min_y = profiles
        .iter()
        .flat_map(|p| p.chart_data.seven_day_points.iter())
        .map(|pt| pt.y)
        .fold(f64::INFINITY, f64::min);
    if min_y.is_infinite() {
        return 0.0;
    }
    (min_y / 5.0).floor() * 5.0
}
```

- [ ] **Step 2: `App::load()` 用 `auto_y_lower` 初始化**

```rust
let y_zoom_lower = auto_y_lower(&profiles);
Ok(Self {
    // ...
    y_zoom_lower,
    // ...
})
```

- [ ] **Step 3: `reload_profiles` 更新後同步 y_zoom_lower**

在 `reload_profiles` 裡，reload 完成後：
```rust
self.profiles = load_profiles(...)?;
self.y_zoom_lower = auto_y_lower(&self.profiles);
// ...
```

**注意：** 這樣每次 reload（包含 30s auto-reload）都會重新計算 auto fit，可能在使用者手動 zoom 後被覆蓋。為避免干擾，只在 y_zoom_lower == auto_y_lower (舊 profiles) 時才更新（即使用者沒有手動調整過才 auto-fit）。

更安全的做法：加一個 `y_zoom_user_adjusted: bool`：
```rust
// App struct 加：
y_zoom_user_adjusted: bool,
```
初始為 `false`。ZoomIn/ZoomOut 設為 `true`；ResetZoom 設回 `false`。reload_profiles 只在 `!self.y_zoom_user_adjusted` 時更新 y_zoom_lower。

```rust
// reload_profiles 裡：
self.profiles = load_profiles(...)?;
if !self.y_zoom_user_adjusted {
    self.y_zoom_lower = auto_y_lower(&self.profiles);
}

// handle_action ZoomIn/ZoomOut：
InputAction::ZoomIn => {
    self.y_zoom_lower = (self.y_zoom_lower + 5.0).min(95.0);
    self.y_zoom_user_adjusted = true;
}
InputAction::ZoomOut => {
    self.y_zoom_lower = (self.y_zoom_lower - 5.0).max(0.0);
    self.y_zoom_user_adjusted = true;
}
InputAction::ResetZoom => {
    self.y_zoom_lower = auto_y_lower(&self.profiles);
    self.y_zoom_user_adjusted = false;
}
```

Also init `y_zoom_user_adjusted: false` in both `load()` and `from_profile_names()`.

- [ ] **Step 4: 確認所有測試通過**

```bash
cd rust/plot-viewer && cargo test 2>&1
```
Expected: 全部通過，0 warnings。

- [ ] **Step 5: Commit**

```bash
git add rust/plot-viewer/src/app.rs
git commit -m "feat(chart): Y-axis auto-fit to data min on startup; r=reset to auto-fit"
```

---

## Verification Checklist

```bash
cd rust/plot-viewer
cargo build
cargo test
cargo run
```

目視確認：
- [ ] Accounts pane：Enter/n/d/u 有效；Plot pane 無效
- [ ] `/` 在 Accounts pane 進入 filter，title 顯示 `Profiles [/keyword]`，即時過濾清單；Esc 清除
- [ ] Tab 切到 Plot pane：`1`/`3`/`7` 改變 X 軸視窗和 labels
- [ ] `s` 在 Plot pane toggle solo，只顯示 selected series
- [ ] `←→` 在 Plot pane 移動 DarkGray 游標線，band summary 第三行顯示游標位置
- [ ] `r` 重置 Y 到 auto-fit；`+/-` 手動 zoom 後 `r` 恢復
- [ ] Details 的 `reset in` 顯示 `3d 9h` 格式而非秒數
- [ ] Context footer 在 Accounts pane 和 Plot pane 顯示不同內容
- [ ] 30s 後（或 cron 更新後）chart 自動 refresh（不強制 fetch）
- [ ] 0 warnings
