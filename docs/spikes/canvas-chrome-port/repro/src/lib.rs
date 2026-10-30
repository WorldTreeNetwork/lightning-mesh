//! Isolated API probe. Does not create a window.
//!
//! Type-checks the Papyrus D-043 `WebviewWindowBuilder` chain against
//! `tauri` 2.11.5 with `macos-private-api`. CSS ticks live in HTML and
//! are not part of this compile.

use tauri::{Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

#[cfg(target_os = "macos")]
use tauri::window::{Effect, EffectState, EffectsBuilder};

/// Overlay builder chain copied from Papyrus `src-tauri/src/canvas.rs`
/// (`open_with`, decorations/transparent/shadow/HudWindow).
pub fn probe_overlay_builder<R: Runtime, M: Manager<R>>(
    manager: &M,
) -> WebviewWindowBuilder<'_, R, M> {
    let mut builder = WebviewWindowBuilder::new(
        manager,
        "canvas-spike",
        WebviewUrl::App("index.html".into()),
    )
    .title("")
    .inner_size(320.0, 220.0)
    .position(96.0, 72.0)
    .decorations(false)
    .transparent(true)
    .background_color(tauri::window::Color(0, 0, 0, 0))
    .shadow(false)
    .resizable(true)
    .always_on_top(true)
    .visible(false);

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

    builder
}
