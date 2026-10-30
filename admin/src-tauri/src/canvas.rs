// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 World Tree Network Foundation and the Lightning Mesh contributors

//! Owner-drawn canvas overlay (ported from Papyrus `canvas.rs`).
//!
//! One canvas in this landing: a fixed label, reused and focused when it is
//! already open. No OS title bar, shadow, or traffic lights; the page draws
//! the chrome. The page fills the well from an allowlisted host source.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

#[cfg(target_os = "macos")]
use tauri::window::{Effect, EffectState, EffectsBuilder};

pub const LABEL_PREFIX: &str = "canvas-";
/// The single canvas of this landing.
pub const CANVAS_ID: &str = "canvas-well";

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

/// Tracks open canvas ids. At most one entry in this landing.
#[derive(Default)]
pub struct CanvasState {
    ids: Mutex<Vec<String>>,
    pub(crate) source: crate::stdout::SourceState,
}

impl CanvasState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record `id`; returns false when it was already tracked.
    fn insert(&self, id: &str) -> bool {
        let Ok(mut g) = self.ids.lock() else {
            return false;
        };
        if g.iter().any(|x| x == id) {
            return false;
        }
        g.push(id.to_string());
        self.source.activate();
        true
    }

    fn remove(&self, id: &str) {
        if let Ok(mut g) = self.ids.lock() {
            g.retain(|x| x != id);
            self.source.deactivate();
        }
    }

    fn snapshot(&self) -> Vec<String> {
        self.ids.lock().map(|g| g.clone()).unwrap_or_default()
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
    open_with(&app, None)
}

#[tauri::command]
pub fn close_canvas<R: Runtime>(app: AppHandle<R>, id: String) -> Result<(), CanvasError> {
    if !is_canvas_label(&id) {
        return Err(CanvasError::Invalid("not a canvas id".into()));
    }
    if let Some(state) = app.try_state::<CanvasState>() {
        state.remove(&id);
    }
    if let Some(win) = app.get_webview_window(&id) {
        win.close().map_err(|e| CanvasError::Window(e.to_string()))?;
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
    match open_with(app, None) {
        Ok(h) => tracing::info!(target: "canvas", "env canvas opened {}", h.id),
        Err(e) => tracing::warn!(target: "canvas", "env canvas open failed: {e}"),
    }
}

fn open_with<R: Runtime>(
    app: &AppHandle<R>,
    spec: Option<CanvasSpec>,
) -> Result<CanvasHandle, CanvasError> {
    // Reuse: a second open focuses the live canvas instead of stacking.
    if let Some(win) = app.get_webview_window(CANVAS_ID) {
        let _ = win.show();
        let _ = win.set_focus();
        if let Some(h) = live_handle(app, CANVAS_ID) {
            return Ok(h);
        }
    }
    let spec = spec.unwrap_or(CanvasSpec {
        x: 220.0,
        y: 140.0,
        width: DEFAULT_W,
        height: DEFAULT_H,
    });
    validate_spec(&spec)?;

    let init = format!(
        "window.__LIGHTNING_CANVAS_ID = '{CANVAS_ID}';\
         window.__LIGHTNING_CANVAS_CHROME = {{cut:{CHAMFER},bevel:{BEVEL},facet:{FACET_DEG}}};"
    );

    let mut builder =
        WebviewWindowBuilder::new(app, CANVAS_ID, WebviewUrl::App("canvas.html".into()))
            .title("")
            .inner_size(spec.width, spec.height)
            .position(spec.x, spec.y)
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

    // Register before the page can invoke its on-load fill.
    if let Some(state) = app.try_state::<CanvasState>() {
        state.insert(CANVAS_ID);
    }
    let win = builder.build().map_err(|e| {
        if let Some(state) = app.try_state::<CanvasState>() {
            state.remove(CANVAS_ID);
        }
        CanvasError::Window(e.to_string())
    })?;
    clip_silhouette(&win);
    let _ = win.show();
    let _ = win.set_focus();

    Ok(live_handle(app, CANVAS_ID).unwrap_or(CanvasHandle {
        id: CANVAS_ID.to_string(),
        x: spec.x,
        y: spec.y,
        width: spec.width,
        height: spec.height,
    }))
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
        if let (Some(shape_cls), Some(color_cls)) = (
            AnyClass::get(c"CAShapeLayer"),
            AnyClass::get(c"NSColor"),
        ) {
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
        CanvasSpec { x: 0.0, y: 0.0, width: w, height: h }
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
    fn one_canvas_reuses_id() {
        let st = CanvasState::new();
        assert!(st.insert(CANVAS_ID));
        assert!(!st.insert(CANVAS_ID));
        assert_eq!(st.snapshot(), vec![CANVAS_ID.to_string()]);
        st.remove(CANVAS_ID);
        assert!(st.snapshot().is_empty());
        assert!(is_canvas_label(CANVAS_ID));
        assert!(!is_canvas_label("main"));
    }
}
