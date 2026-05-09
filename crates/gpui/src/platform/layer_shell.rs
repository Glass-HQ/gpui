//! Wayland layer-shell window configuration.

use crate::Pixels;
use bitflags::bitflags;

/// The Wayland layer a layer-shell surface should occupy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Layer {
    /// Render behind normal windows, typically for wallpapers.
    Background,
    /// Render below normal windows but above the background layer.
    Bottom,
    /// Render above normal windows.
    #[default]
    Top,
    /// Render above all other layers.
    Overlay,
}

bitflags! {
    /// Edge anchors for a Wayland layer-shell surface.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Anchor: u32 {
        /// Anchor to the top edge of the output.
        const TOP = 1 << 0;
        /// Anchor to the bottom edge of the output.
        const BOTTOM = 1 << 1;
        /// Anchor to the left edge of the output.
        const LEFT = 1 << 2;
        /// Anchor to the right edge of the output.
        const RIGHT = 1 << 3;
    }
}

/// Keyboard focus behavior for a Wayland layer-shell surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyboardInteractivity {
    /// Never receive keyboard focus.
    #[default]
    None,
    /// Exclusively receive keyboard focus when shown.
    Exclusive,
    /// Receive keyboard focus only when demanded by the compositor.
    OnDemand,
}

/// Options for creating a Wayland layer-shell window.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LayerShellOptions {
    /// The namespace reported to the compositor for this surface.
    pub namespace: String,
    /// Which compositor layer should host the surface.
    pub layer: Layer,
    /// Which output edges the surface should stay attached to.
    pub anchor: Anchor,
    /// Optional top, right, bottom, left margins.
    pub margin: Option<(Pixels, Pixels, Pixels, Pixels)>,
    /// Optional compositor-reserved exclusive zone.
    pub exclusive_zone: Option<Pixels>,
    /// Optional edge used for exclusive zone calculations.
    pub exclusive_edge: Option<Anchor>,
    /// How keyboard focus should be handled.
    pub keyboard_interactivity: KeyboardInteractivity,
}
