# Unify Details Panel Implementation Plan

> **Status:** Partially archived. The large structural refactor described here has mostly shipped: side panels are gone, chart rendering is full-width, and the left pane already combines profile list + details. However, this document still contains potentially useful polish ideas for final wording/layout cleanup after real-account verification.
>
> **Roadmap note:** Use this as a follow-up reference, not as a literal task list. Anything below that assumes `render/panels.rs` still exists is superseded by the current architecture.

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 將 Details/Summary/Compare 整合到左下單一 Details 區塊，移除右側 panels，chart 佔滿右側全寬，profiles 清單動態顯示 3-10 行。

**Architecture:** 移除 `render/mod.rs` 的 68/32 水平分割，chart 直接填滿右側 70% 區域；左側 `render_left_pane()` 改為動態 profiles 行數 + 統一 details；`panels.rs` 的 `draw_panels`/`render_panels` 及其測試全部移除。

**Tech Stack:** Rust, Ratatui, Crossterm

---

## Current reality snapshot

- `rust/plot-viewer/src/render/panels.rs` has already been removed.
- The chart now fills the full right pane via `render::render(..., area)`.
- The left pane already renders dynamic profile list height plus a unified details block.
- Remaining polish should focus on details wording, field ordering, and any UX issues surfaced by live Codex/Claude verification.

---

## 目標畫面

```
┌─ Profiles ────────┬─ usage plot overlays ────────────────────────┐
│ ▶ alpha [saved]   │                                              │
│   beta  [saved]   │        ····                                  │
│   gamma [unsaved] │   ··         ·····                           │
├─ Details ─────────│                                              │
│ Profile: alpha    │                                              │
│ State: current    │                                              │
│ Last updated: 2m  │                                              │
│ Email: a@b.com    │                                              │
│ Plan: plus        │                                              │
│ Weekly: 45% used  │                                              │
│ 5h: 12% used      │                                              │
├───────────────────┴──────────────────────────────────────────────┤
│ Enter=switch · n=rename · d=delete · Tab=pane · Space=focus      │
└──────────────────────────────────────────────────────────────────┘
```

## 欄位規格

**Details 顯示欄位（順序如下）：**
| 欄位 | 條件 | 格式 |
|------|------|------|
| Profile | 永遠 | `Profile: <name>` |
| State | 永遠 | `State: current \| saved \| unsaved` |
| Last updated | 永遠 | `Last updated: 2m ago` / `stale 5h ago` / `never` |
| Email | 有 usage | `Email: <email>` |
| Plan | 有 usage | `Plan: <plan_type>` |
| Weekly | 有 usage | `Weekly: XX% used, reset in Xs` |
| 5h | 有 usage | `5h:     XX% used, reset in Xs` |
| (blank) | 有 status | `` |
| Status msg | 有 status | `<message>` |

**移除欄位：** Account ID, Usage source, Compare 區塊, Summary 區塊 (Visible profiles, 7d samples, 5h band text, Focus panel, Snapshot current)

## 檔案異動

| 檔案 | 動作 |
|------|------|
| `rust/plot-viewer/src/render/mod.rs` | Modify: 移除 68/32 split 和 panels call，chart 填滿 area |
| `rust/plot-viewer/src/render/panels.rs` | Delete: 整個模組移除（含測試） |
| `rust/plot-viewer/src/render/mod.rs` | Modify: 移除 `pub mod panels` |
| `rust/plot-viewer/src/app.rs` | Modify: `render_left_pane()`, `render_account_detail()`, 新增 `format_age()` |
| `rust/plot-viewer/tests/rust_cli_migration.rs` | Modify: 確認無 panels 相關引用 |

---

## Chunk 1: render/mod.rs — 移除 panels，chart 全寬

### Task 1: 移除右側 panels，chart 佔滿右側

**Files:**
- Modify: `rust/plot-viewer/src/render/mod.rs`

目前 `render()` 函式（簡化後）：
```rust
pub fn render<State: RenderState>(frame: &mut Frame, area: Rect, state: &State) {
    let context = RenderContext::new(state, area);
    if area.width == 0 || area.height == 0 { return; }
    let body = Layout::horizontal([Constraint::Percentage(68), Constraint::Percentage(32)])
        .split(area);
    chart::render_chart(frame, &context.with_area(body[0]));
    panels::draw_panels(frame, &context.with_area(body[1]));
}
```

- [ ] **Step 1: 移除 `pub mod panels` 宣告與 panels 相關 use**

在 `render/mod.rs` 頂端：
```rust
// 移除這行：
pub mod panels;
```

- [ ] **Step 2: 更新 `render()` 函式**

```rust
pub fn render<State: RenderState>(frame: &mut Frame, area: Rect, state: &State) {
    let context = RenderContext::new(state, area);
    if area.width == 0 || area.height == 0 {
        return;
    }
    chart::render_chart(frame, &context.with_area(area));
}
```

- [ ] **Step 3: 刪除 `rust/plot-viewer/src/render/panels.rs`**

```bash
rm rust/plot-viewer/src/render/panels.rs
```

- [ ] **Step 4: 確認編譯**

```bash
cd rust/plot-viewer && cargo build 2>&1
```
Expected: 無 error。可能有 warning（`SelectionState` 等 struct 暫時 unused — Task 2 會清理）

- [ ] **Step 5: Commit**

```bash
git add rust/plot-viewer/src/render/mod.rs rust/plot-viewer/src/render/panels.rs
git commit -m "refactor(render): remove side panels, chart fills full right area"
```

---

## Chunk 2: app.rs — 統一 Details + 動態 profiles 高度

### Task 2: 新增 `format_age()` helper

**Files:**
- Modify: `rust/plot-viewer/src/app.rs`

- [ ] **Step 1: 新增 `use std::time::{SystemTime, UNIX_EPOCH}` import（若尚未有）**

確認 app.rs 頂端已有：
```rust
use std::time::Duration; // 已有
```
加入（若未有）：
```rust
use std::time::{SystemTime, UNIX_EPOCH};
```

- [ ] **Step 2: 在 `summarize_window` 附近加入 `format_age()`**

```rust
fn format_age(fetched_at: Option<u64>, stale: bool) -> String {
    let Some(ts) = fetched_at else {
        return "never".to_string();
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let age_secs = now.saturating_sub(ts);
    let age_str = if age_secs < 60 {
        format!("{}s ago", age_secs)
    } else if age_secs < 3600 {
        format!("{}m ago", age_secs / 60)
    } else if age_secs < 86400 {
        format!("{}h ago", age_secs / 3600)
    } else {
        format!("{}d ago", age_secs / 86400)
    };
    if stale { format!("stale {}", age_str) } else { age_str }
}
```

- [ ] **Step 3: 更新 `render_account_detail()`**

將現有函式替換為：
```rust
fn render_account_detail(
    profile: Option<&ProfileEntry>,
    status_message: Option<&str>,
) -> Vec<Line<'static>> {
    let Some(profile) = profile else {
        return vec![
            Line::from("No Codex auth profile loaded."),
            Line::from("Save or switch an account to continue."),
        ];
    };

    let state_label = match (profile.is_current, profile.saved_name.is_some()) {
        (true, _) => "current",
        (false, true) => "saved",
        (false, false) => "unsaved",
    };

    let mut lines = vec![
        Line::from(format!("Profile: {}", profile.profile_name)),
        Line::from(format!("State: {}", state_label)),
        Line::from(format!(
            "Last updated: {}",
            format_age(profile.usage_view.fetched_at, profile.usage_view.stale)
        )),
    ];

    if let Some(usage) = profile.usage_view.usage.as_ref() {
        if let Some(email) = usage.email.as_deref() {
            lines.push(Line::from(format!("Email: {}", email)));
        }
        if let Some(plan) = usage.plan_type.as_deref() {
            lines.push(Line::from(format!("Plan:  {}", plan)));
        }
        if let Some(w) = pick_weekly_window(usage) {
            lines.push(Line::from(format!(
                "Weekly: {:.0}% used, reset in {}s",
                w.used_percent, w.reset_after_seconds
            )));
        }
        if let Some(w) = pick_five_hour_window(usage) {
            lines.push(Line::from(format!(
                "5h:     {:.0}% used, reset in {}s",
                w.used_percent, w.reset_after_seconds
            )));
        }
    }

    if let Some(message) = status_message {
        lines.push(Line::from(""));
        lines.push(Line::from(message.to_string()));
    }

    lines
}
```

**注意：** 移除了 `Account:` 和 `Usage source:` 行，拆分 Email/Plan 為獨立行，`Weekly`/`5h` 用 `{:.0}%` 格式。

- [ ] **Step 4: 更新 `render_left_pane()` — 動態 profiles 高度**

```rust
fn render_left_pane(&self, frame: &mut Frame, area: Rect) {
    // profiles 列表高度：內容行數 clamp 到 3..=10，再加 2（上下邊框）
    let list_lines = (self.profiles.len().max(3).min(10) + 2) as u16;

    let [list_area, detail_area] =
        Layout::vertical([Constraint::Length(list_lines), Constraint::Min(0)]).areas(area);

    let profiles_title = if self.pane_focus == PaneFocus::Accounts {
        "Profiles [active]"
    } else {
        "Profiles"
    };
    let items = self
        .profiles
        .iter()
        .map(|profile| {
            let prefix = if profile.is_current { "▶" } else { " " };
            let saved = if profile.saved_name.is_some() { "saved" } else { "unsaved" };
            let usage = format_usage_badge(&profile.usage_view);
            ListItem::new(format!("{prefix} {} [{saved}] {usage}", profile.profile_name))
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    state.select((!self.profiles.is_empty()).then_some(self.selected_profile_index));
    let list = List::new(items)
        .block(Block::default().title(profiles_title).borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, list_area, &mut state);

    let detail_lines =
        render_account_detail(self.selected_profile(), self.status_message.as_deref());
    let details = Paragraph::new(Text::from(detail_lines))
        .block(Block::default().title("Details").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    frame.render_widget(details, detail_area);
}
```

**關鍵：** `list_lines = clamp(profile_count, 3, 10) + 2`，Details 用 `Constraint::Min(0)` 填滿剩餘空間。

- [ ] **Step 5: 確認 `pick_weekly_window` / `pick_five_hour_window` 仍存在**

```bash
grep -n "fn pick_weekly_window\|fn pick_five_hour_window" rust/plot-viewer/src/app.rs
```
Expected: 找到兩個函式定義。

- [ ] **Step 6: 移除 `summarize_window()` 函式（已不再使用）**

在 app.rs 中找到並刪除：
```rust
fn summarize_window(window: Option<&UsageWindow>) -> String { ... }
```

- [ ] **Step 7: 確認 app.rs 不再引用 panels**

```bash
grep -n "panels" rust/plot-viewer/src/app.rs
```
Expected: 無輸出。

- [ ] **Step 8: 確認編譯與測試**

```bash
cd rust/plot-viewer && cargo test 2>&1
```
Expected: 所有測試通過，無 error，無 warning。

- [ ] **Step 9: Commit**

```bash
git add rust/plot-viewer/src/app.rs
git commit -m "feat(ui): unify Details panel with age display, dynamic profiles 3-10 lines"
```

---

## Chunk 3: 清理 unused 符號

### Task 3: 清理 render/mod.rs unused structs 與 app.rs unused imports

**Files:**
- Modify: `rust/plot-viewer/src/render/mod.rs`
- Modify: `rust/plot-viewer/src/app.rs`

`panels.rs` 移除後，`render/mod.rs` 中的部分 struct/trait 可能只被 panels 使用。

- [ ] **Step 1: 確認 `render/mod.rs` 中哪些 pub 型別不再被外部使用**

```bash
cd rust/plot-viewer && cargo build 2>&1 | grep "warning.*unused\|warning.*never"
```

- [ ] **Step 2: 根據 warning 移除對應 dead code**

常見候選（只有在確認 warning 時才移除）：
- `PanelSkeleton` struct（已在 panels.rs，隨之刪除）
- `SelectionState`、`ChartState` 等若只被 panels 使用

- [ ] **Step 3: 最終確認**

```bash
cd rust/plot-viewer && cargo test 2>&1
```
Expected: 所有測試通過，0 warnings，0 errors。

- [ ] **Step 4: Commit**

```bash
git add -p
git commit -m "chore(render): remove unused types after panels removal"
```

---

## Verification Checklist

```bash
cd rust/plot-viewer

# 1. 編譯
cargo build

# 2. 測試
cargo test

# 3. 目視確認（需要 terminal）
cargo run
```

目視確認項目：
- [ ] 左側 Profiles 清單：1-2 個帳號時顯示 3 行，多帳號最多 10 行
- [ ] 左側 Details：顯示 Profile / State / Last updated / Email / Plan / Weekly / 5h
- [ ] 右側：只有 chart，無 Summary/Compare 邊框
- [ ] Tab 切換 pane、Space 切換 inner focus 仍正常
- [ ] 0 warnings
