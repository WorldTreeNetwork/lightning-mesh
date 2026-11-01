// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! Owner-drawn canvas overlay (ported from Papyrus `canvas.rs`).
//!
//! Up to four independent wells, placed in a session-only grid on opening.
//! No OS title bar, shadow, or traffic lights; the page draws the chrome.
//! Each window owns its allowlisted host source and cancellation generation.

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

#[cfg(target_os = "macos")]
use tauri::window::{Effect, EffectState, EffectsBuilder};

pub const LABEL_PREFIX: &str = "canvas-";
pub const MAX_CANVASES: usize = 4;
const CELL_GAP: f64 = 16.0;

pub const DEFAULT_W: f64 = 320.0;
pub const DEFAULT_H: f64 = 220.0;
pub const MIN_SIZE: f64 = 80.0;
pub const MAX_SIZE: f64 = 4096.0;
pub const CHAMFER: f64 = 18.0;
pub const BEVEL: f64 = 7.0;
pub const FACET_DEG: f64 = 45.0;

#[derive(Debug, Clone, Deserialize)]
pub struct CanvasSpec {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CanvasHandle {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug)]
pub enum CanvasError {
    Invalid(String),
    Window(String),
}

impl std::fmt::Display for CanvasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CanvasError::Invalid(s) | CanvasError::Window(s) => f.write_str(s),
        }
    }
}

impl Serialize for CanvasError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

struct Well {
    initial: CanvasHandle,
    source: Arc<crate::stdout::SourceState>,
}

#[derive(Default)]
struct Wells {
    next_id: u64,
    entries: Vec<Well>,
}

/// Reservations count toward the limit before their webviews can start filling.
#[derive(Default)]
pub struct CanvasState {
    wells: Mutex<Wells>,
}

impl CanvasState {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn reserve(
        &self,
        screen: &CanvasSpec,
        latest_live: Option<&CanvasHandle>,
    ) -> Result<CanvasHandle, CanvasError> {
        let mut wells = self.wells.lock().unwrap_or_else(|e| e.into_inner());
        if wells.entries.len() >= MAX_CANVASES {
            return Err(CanvasError::Invalid(
                "Four canvas wells are already open. Close one before opening another.".into(),
            ));
        }
        // If another open reserved a slot while live geometry was being read,
        // place after that reservation rather than using stale geometry.
        let previous = wells.entries.last().map(|well| {
            latest_live
                .filter(|live| live.id == well.initial.id)
                .unwrap_or(&well.initial)
        });
        let spec = next_spec(screen, previous);
        validate_spec(&spec)?;
        wells.next_id += 1;
        let handle = CanvasHandle {
            id: format!("{LABEL_PREFIX}{}", wells.next_id),
            x: spec.x,
            y: spec.y,
            width: spec.width,
            height: spec.height,
        };
        let source = Arc::new(crate::stdout::SourceState::default());
        source.activate();
        wells.entries.push(Well {
            initial: handle.clone(),
            source,
        });
        Ok(handle)
    }

    pub(crate) fn source(&self, id: &str) -> Result<Arc<crate::stdout::SourceState>, String> {
        self.wells
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entries
            .iter()
            .find(|well| well.initial.id == id)
            .map(|well| Arc::clone(&well.source))
            .ok_or_else(|| "Canvas well is not open.".into())
    }

    pub(crate) fn remove(&self, id: &str) {
        let removed = {
            let mut wells = self.wells.lock().unwrap_or_else(|e| e.into_inner());
            wells
                .entries
                .iter()
                .position(|well| well.initial.id == id)
                .map(|index| wells.entries.remove(index))
        };
        if let Some(well) = removed {
            well.source.deactivate();
        }
    }

    fn snapshot(&self) -> Vec<String> {
        self.wells
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entries
            .iter()
            .map(|well| well.initial.id.clone())
            .collect()
    }
}

/// Only initial placement is gridded. Moving or resizing never calls this.
fn next_spec(screen: &CanvasSpec, previous: Option<&CanvasHandle>) -> CanvasSpec {
    let width = DEFAULT_W.min(screen.width);
    let height = DEFAULT_H.min(screen.height);
    let (mut x, mut y) = match previous {
        Some(previous) => (previous.x + previous.width + CELL_GAP, previous.y),
        None => (screen.x + 220.0, screen.y + 140.0),
    };
    if let Some(previous) = previous {
        if x + width > screen.x + screen.width {
            x = screen.x;
            y = previous.y + previous.height + CELL_GAP;
        }
    }
    CanvasSpec {
        x: x.clamp(screen.x, screen.x + screen.width - width),
        y: y.clamp(screen.y, screen.y + screen.height - height),
        width,
        height,
    }
}

/// Reject non-finite values and sizes outside `[MIN_SIZE, MAX_SIZE]`.
pub fn validate_spec(spec: &CanvasSpec) -> Result<(), CanvasError> {
    for (name, v) in [
        ("x", spec.x),
        ("y", spec.y),
        ("width", spec.width),
        ("height", spec.height),
    ] {
        if !v.is_finite() {
            return Err(CanvasError::Invalid(format!("{name} must be finite")));
        }
    }
    if spec.width < MIN_SIZE || spec.height < MIN_SIZE {
        return Err(CanvasError::Invalid(format!(
            "size must be at least {MIN_SIZE}×{MIN_SIZE}"
        )));
    }
    if spec.width > MAX_SIZE || spec.height > MAX_SIZE {
        return Err(CanvasError::Invalid(format!(
            "size must be at most {MAX_SIZE}×{MAX_SIZE}"
        )));
    }
    Ok(())
}

fn is_canvas_label(label: &str) -> bool {
    label.starts_with(LABEL_PREFIX)
}

#[tauri::command]
pub fn open_canvas<R: Runtime>(app: AppHandle<R>) -> Result<CanvasHandle, CanvasError> {
    open_with(&app)
}

#[tauri::command]
pub fn close_canvas<R: Runtime>(window: tauri::WebviewWindow<R>) -> Result<(), CanvasError> {
    let id = window.label();
    if !is_canvas_label(id) {
        return Err(CanvasError::Invalid("not a canvas id".into()));
    }
    window
        .close()
        .map_err(|e| CanvasError::Window(e.to_string()))?;
    if let Some(state) = window.app_handle().try_state::<CanvasState>() {
        state.remove(id);
    }
    Ok(())
}

#[tauri::command]
pub fn list_canvases<R: Runtime>(app: AppHandle<R>) -> Vec<CanvasHandle> {
    let Some(state) = app.try_state::<CanvasState>() else {
        return Vec::new();
    };
    state
        .snapshot()
        .iter()
        .filter_map(|id| live_handle(&app, id))
        .collect()
}

/// Verify hook: `LIGHTNING_ADMIN_OPEN_CANVAS=1` opens the canvas at startup.
pub fn open_if_env<R: Runtime>(app: &AppHandle<R>) {
    if std::env::var_os("LIGHTNING_ADMIN_OPEN_CANVAS").is_none() {
        return;
    }
    match open_with(app) {
        Ok(h) => tracing::info!(target: "canvas", "env canvas opened {}", h.id),
        Err(e) => tracing::warn!(target: "canvas", "env canvas open failed: {e}"),
    }
}

fn open_with<R: Runtime>(app: &AppHandle<R>) -> Result<CanvasHandle, CanvasError> {
    let state = app.state::<CanvasState>();
    let latest_id = state.snapshot().pop();
    let latest_live = latest_id.as_deref().and_then(|id| live_handle(app, id));
    let anchor = latest_id
        .as_deref()
        .and_then(|id| app.get_webview_window(id))
        .or_else(|| app.get_webview_window("main"));
    let monitor = anchor
        .and_then(|win| win.current_monitor().ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten())
        .ok_or_else(|| CanvasError::Window("Cannot find a screen for a canvas well.".into()))?;
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let screen = CanvasSpec {
        x: area.position.x as f64 / scale,
        y: area.position.y as f64 / scale,
        width: area.size.width as f64 / scale,
        height: area.size.height as f64 / scale,
    };
    // Register before the page can invoke its on-load fill. The same lock
    // enforces the cap for simultaneous open requests.
    let handle = state.reserve(&screen, latest_live.as_ref())?;
    let id = &handle.id;

    let init = format!(
        "window.__LIGHTNING_CANVAS_CHROME = {{cut:{CHAMFER},bevel:{BEVEL},facet:{FACET_DEG}}};"
    );

    let mut builder = WebviewWindowBuilder::new(app, id, WebviewUrl::App("canvas.html".into()))
        .title("")
        .inner_size(handle.width, handle.height)
        .position(handle.x, handle.y)
        .decorations(false)
        .transparent(true)
        .background_color(tauri::window::Color(0, 0, 0, 0))
        .shadow(false)
        .resizable(true)
        .min_inner_size(MIN_SIZE, MIN_SIZE)
        .max_inner_size(MAX_SIZE, MAX_SIZE)
        .skip_taskbar(true)
        .always_on_top(true)
        .visible(false)
        .focused(true)
        .accept_first_mouse(true)
        .initialization_script(init);

    #[cfg(target_os = "macos")]
    {
        builder = builder.hidden_title(true).effects(
            EffectsBuilder::new()
                .effect(Effect::HudWindow)
                .effect(Effect::Acrylic)
                .state(EffectState::Active)
                .radius(0.0)
                .build(),
        );
    }

    let win = builder.build().map_err(|e| {
        state.remove(id);
        CanvasError::Window(e.to_string())
    })?;
    clip_silhouette(&win);
    let _ = win.show();
    let _ = win.set_focus();

    Ok(live_handle(app, id).unwrap_or(handle))
}

pub fn on_resized<R: Runtime>(app: &AppHandle<R>, label: &str) {
    if !is_canvas_label(label) {
        return;
    }
    if let Some(win) = app.get_webview_window(label) {
        clip_silhouette(&win);
    }
}

pub fn on_destroyed<R: Runtime>(app: &AppHandle<R>, label: &str) {
    if !is_canvas_label(label) {
        return;
    }
    if let Some(state) = app.try_state::<CanvasState>() {
        state.remove(label);
    }
}

fn live_handle<R: Runtime>(app: &AppHandle<R>, id: &str) -> Option<CanvasHandle> {
    let win = app.get_webview_window(id)?;
    let scale = win.scale_factor().unwrap_or(1.0);
    let pos = win.outer_position().ok()?;
    let size = win.inner_size().ok()?;
    Some(CanvasHandle {
        id: id.to_string(),
        x: pos.x as f64 / scale,
        y: pos.y as f64 / scale,
        width: size.width as f64 / scale,
        height: size.height as f64 / scale,
    })
}

/// macOS: HudWindow paints a rounded plate even at radius 0. Square every
/// layer and mask the content and frost views to the octagon.
fn clip_silhouette<R: Runtime>(win: &tauri::WebviewWindow<R>) {
    #[cfg(target_os = "macos")]
    unsafe {
        use objc2::runtime::{AnyClass, AnyObject};
        let Ok(ptr) = win.ns_window() else {
            return;
        };
        let ns_win = ptr as *mut AnyObject;
        let content: *mut AnyObject = objc2::msg_send![ns_win, contentView];
        if content.is_null() {
            return;
        }
        let bounds: objc2_foundation::NSRect = objc2::msg_send![content, bounds];
        let (w, h) = (bounds.size.width, bounds.size.height);
        if w < 8.0 || h < 8.0 {
            return;
        }
        let cut_x = CHAMFER.min(w / 2.0 - 1.0).max(0.0);
        let cut_y = (CHAMFER * FACET_DEG.to_radians().tan())
            .min(h / 2.0 - 1.0)
            .max(0.0);
        let pts = [
            (cut_x, h),
            (w - cut_x, h),
            (w, h - cut_y),
            (w, cut_y),
            (w - cut_x, 0.0),
            (cut_x, 0.0),
            (0.0, cut_y),
            (0.0, h - cut_y),
        ];
        let path = CGPathCreateMutable();
        if path.is_null() {
            return;
        }
        CGPathMoveToPoint(path, std::ptr::null(), pts[0].0, pts[0].1);
        for p in pts.iter().skip(1) {
            CGPathAddLineToPoint(path, std::ptr::null(), p.0, p.1);
        }
        CGPathCloseSubpath(path);
        if let (Some(shape_cls), Some(color_cls)) =
            (AnyClass::get(c"CAShapeLayer"), AnyClass::get(c"NSColor"))
        {
            square_layers(content);
            stamp_octagon_mask(content, path, bounds, shape_cls, color_cls);
            if let Some(cls) = AnyClass::get(c"NSVisualEffectView") {
                stamp_class_tree(content, cls, path, bounds, shape_cls, color_cls);
            }
        }
        CGPathRelease(path);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = win;
    }
}

#[cfg(target_os = "macos")]
unsafe fn square_layers(view: *mut objc2::runtime::AnyObject) {
    use objc2::runtime::AnyObject;
    if view.is_null() {
        return;
    }
    let _: () = objc2::msg_send![view, setWantsLayer: true];
    let layer: *mut AnyObject = objc2::msg_send![view, layer];
    if !layer.is_null() {
        let _: () = objc2::msg_send![layer, setCornerRadius: 0.0f64];
    }
    let subviews: *mut AnyObject = objc2::msg_send![view, subviews];
    if subviews.is_null() {
        return;
    }
    let n: usize = objc2::msg_send![subviews, count];
    for i in 0..n {
        let child: *mut AnyObject = objc2::msg_send![subviews, objectAtIndex: i];
        square_layers(child);
    }
}

#[cfg(target_os = "macos")]
unsafe fn stamp_class_tree(
    view: *mut objc2::runtime::AnyObject,
    cls: &objc2::runtime::AnyClass,
    path: *mut std::ffi::c_void,
    bounds: objc2_foundation::NSRect,
    shape_cls: &objc2::runtime::AnyClass,
    color_cls: &objc2::runtime::AnyClass,
) {
    use objc2::runtime::AnyObject;
    if view.is_null() {
        return;
    }
    let is_kind: bool = objc2::msg_send![view, isKindOfClass: cls];
    if is_kind {
        stamp_octagon_mask(view, path, bounds, shape_cls, color_cls);
    }
    let subviews: *mut AnyObject = objc2::msg_send![view, subviews];
    if subviews.is_null() {
        return;
    }
    let n: usize = objc2::msg_send![subviews, count];
    for i in 0..n {
        let child: *mut AnyObject = objc2::msg_send![subviews, objectAtIndex: i];
        stamp_class_tree(child, cls, path, bounds, shape_cls, color_cls);
    }
}

#[cfg(target_os = "macos")]
unsafe fn stamp_octagon_mask(
    view: *mut objc2::runtime::AnyObject,
    path: *mut std::ffi::c_void,
    bounds: objc2_foundation::NSRect,
    shape_cls: &objc2::runtime::AnyClass,
    color_cls: &objc2::runtime::AnyClass,
) {
    use objc2::runtime::AnyObject;
    let _: () = objc2::msg_send![view, setWantsLayer: true];
    let layer: *mut AnyObject = objc2::msg_send![view, layer];
    if layer.is_null() {
        return;
    }
    let mask: *mut AnyObject = objc2::msg_send![shape_cls, layer];
    if mask.is_null() {
        return;
    }
    let _: () = objc2::msg_send![mask, setFrame: bounds];
    let _: () = objc2::msg_send![mask, setPath: path];
    let black: *mut AnyObject = objc2::msg_send![color_cls, blackColor];
    if !black.is_null() {
        let cg: *mut AnyObject = objc2::msg_send![black, CGColor];
        if !cg.is_null() {
            let _: () = objc2::msg_send![mask, setFillColor: cg];
        }
    }
    let _: () = objc2::msg_send![layer, setMask: mask];
    let _: () = objc2::msg_send![layer, setMasksToBounds: true];
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPathCreateMutable() -> *mut std::ffi::c_void;
    fn CGPathMoveToPoint(path: *mut std::ffi::c_void, m: *const std::ffi::c_void, x: f64, y: f64);
    fn CGPathAddLineToPoint(
        path: *mut std::ffi::c_void,
        m: *const std::ffi::c_void,
        x: f64,
        y: f64,
    );
    fn CGPathCloseSubpath(path: *mut std::ffi::c_void);
    fn CGPathRelease(path: *mut std::ffi::c_void);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(w: f64, h: f64) -> CanvasSpec {
        CanvasSpec {
            x: 0.0,
            y: 0.0,
            width: w,
            height: h,
        }
    }

    #[test]
    fn rejects_non_finite() {
        let mut s = spec(200.0, 200.0);
        s.x = f64::NAN;
        assert!(validate_spec(&s).is_err());
        let mut s = spec(200.0, 200.0);
        s.height = f64::INFINITY;
        assert!(validate_spec(&s).is_err());
    }

    #[test]
    fn enforces_min_and_max() {
        assert!(validate_spec(&spec(79.0, 200.0)).is_err());
        assert!(validate_spec(&spec(200.0, 4097.0)).is_err());
        assert!(validate_spec(&spec(MIN_SIZE, MAX_SIZE)).is_ok());
    }

    #[test]
    fn two_canvas_ids_have_independent_lifetimes() {
        let st = CanvasState::new();
        let first = st.reserve(&spec(1920.0, 1080.0), None).unwrap();
        let second = st.reserve(&spec(1920.0, 1080.0), None).unwrap();
        assert_ne!(first.id, second.id);
        assert!(is_canvas_label(&first.id));
        assert!(is_canvas_label(&second.id));
        assert!(!Arc::ptr_eq(
            &st.source(&first.id).unwrap(),
            &st.source(&second.id).unwrap()
        ));
        assert_eq!(st.snapshot(), vec![first.id.clone(), second.id.clone()]);
        st.remove(&first.id);
        st.remove(&first.id); // duplicate Destroyed must not affect the survivor.
        assert!(st.source(&first.id).is_err());
        assert!(st.source("main").is_err());
        assert_eq!(st.snapshot(), vec![second.id.clone()]);
        st.remove(&second.id);
        assert!(st.snapshot().is_empty());
        assert!(!is_canvas_label("main"));
    }

    #[test]
    fn fifth_canvas_is_refused_and_close_frees_a_slot() {
        let st = CanvasState::new();
        let screen = spec(1920.0, 1080.0);
        for _ in 0..MAX_CANVASES {
            st.reserve(&screen, None).unwrap();
        }
        let before = st.snapshot();
        let error = st.reserve(&screen, None).unwrap_err();
        assert!(error
            .to_string()
            .contains("Four canvas wells are already open"));
        assert_eq!(st.snapshot(), before);
        st.remove(&before[0]);
        let replacement = st.reserve(&screen, None).unwrap();
        assert!(!before.contains(&replacement.id));
        assert_eq!(st.snapshot().len(), MAX_CANVASES);
    }

    #[test]
    fn simultaneous_canvas_opens_reserve_at_most_four_slots() {
        let st = Arc::new(CanvasState::new());
        let opens: Vec<_> = (0..8)
            .map(|_| {
                let st = Arc::clone(&st);
                std::thread::spawn(move || st.reserve(&spec(1920.0, 1080.0), None).ok())
            })
            .collect();
        let mut ids: Vec<_> = opens
            .into_iter()
            .filter_map(|open| open.join().unwrap().map(|handle| handle.id))
            .collect();
        assert_eq!(ids.len(), MAX_CANVASES);
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), MAX_CANVASES);
    }

    #[test]
    fn canvas_placement_steps_right_wraps_down_and_clamps() {
        let screen = spec(1000.0, 700.0);
        let st = CanvasState::new();
        let first = st.reserve(&screen, None).unwrap();
        let second = st.reserve(&screen, None).unwrap();
        assert_eq!(
            (second.x, second.y),
            (first.x + DEFAULT_W + CELL_GAP, first.y)
        );
        let third = st.reserve(&screen, None).unwrap();
        assert_eq!(
            (third.x, third.y),
            (screen.x, second.y + DEFAULT_H + CELL_GAP)
        );

        // Use the latest well's live moved/resized rectangle without altering it.
        let moved = CanvasHandle {
            x: 900.0,
            y: 650.0,
            width: 500.0,
            height: 400.0,
            ..third.clone()
        };
        let fourth = st.reserve(&screen, Some(&moved)).unwrap();
        assert_eq!((fourth.x, fourth.y), (screen.x, screen.height - DEFAULT_H));
        assert_eq!(moved.x, 900.0);
        assert_eq!(st.snapshot().len(), MAX_CANVASES);
    }

    #[test]
    fn canvas_placement_uses_live_geometry_and_nonzero_screen_origin() {
        let screen = CanvasSpec {
            x: -1200.0,
            y: 30.0,
            width: 1200.0,
            height: 800.0,
        };
        let st = CanvasState::new();
        let first = st.reserve(&screen, None).unwrap();
        let moved = CanvasHandle {
            x: -1100.0,
            y: 60.0,
            width: 400.0,
            height: 300.0,
            ..first
        };
        let second = st.reserve(&screen, Some(&moved)).unwrap();
        assert_eq!((second.x, second.y), (-684.0, 60.0));
        let small = next_spec(&spec(200.0, 150.0), None);
        assert_eq!(
            (small.x, small.y, small.width, small.height),
            (0.0, 0.0, 200.0, 150.0)
        );
    }
}
