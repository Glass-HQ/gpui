// Failure modes:
// - Device-pixel ratio changes without a resize observer update.
// - Rebinding the DPR media query during its own callback drops the active closure.
// - Zero-sized canvases must still notify GPUI that the window disappeared.
// - Repeated identical observations must not emit redundant resize work.

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CanvasMetrics {
    pub(crate) physical_width: u32,
    pub(crate) physical_height: u32,
    pub(crate) logical_width: f32,
    pub(crate) logical_height: f32,
    pub(crate) scale_factor: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EnvironmentUpdate {
    pub(crate) media_query: Option<String>,
    pub(crate) resize: Option<ResizeUpdate>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ResizeUpdate {
    Hidden {
        scale_factor: f32,
    },
    Visible {
        logical_width: f32,
        logical_height: f32,
        physical_width: u32,
        physical_height: u32,
        scale_factor: f32,
    },
}

#[derive(Debug)]
pub(crate) struct WindowEnvironmentState {
    current_media_query: Option<String>,
    pending_metrics: Option<CanvasMetrics>,
    last_emitted_physical_size: Option<(u32, u32)>,
    last_emitted_scale_factor: Option<f32>,
    dpr_dirty: bool,
}

impl WindowEnvironmentState {
    pub(crate) fn new(initial_dpr: f64) -> Self {
        Self {
            current_media_query: Some(media_query_for_dpr(initial_dpr)),
            pending_metrics: None,
            last_emitted_physical_size: None,
            last_emitted_scale_factor: None,
            dpr_dirty: false,
        }
    }

    pub(crate) fn current_media_query(&self) -> Option<&str> {
        self.current_media_query.as_deref()
    }

    pub(crate) fn queue_resize(&mut self, metrics: CanvasMetrics) {
        self.pending_metrics = Some(metrics);
    }

    pub(crate) fn queue_dpr_change(&mut self) {
        self.dpr_dirty = true;
    }

    pub(crate) fn needs_measurement(&self) -> bool {
        self.dpr_dirty
    }

    pub(crate) fn reconcile(
        &mut self,
        current_dpr: f64,
        measured_metrics: Option<CanvasMetrics>,
        max_texture_dimension: u32,
    ) -> EnvironmentUpdate {
        let next_media_query = media_query_for_dpr(current_dpr);
        let media_query = if self.current_media_query.as_deref() != Some(next_media_query.as_str())
        {
            self.current_media_query = Some(next_media_query.clone());
            Some(next_media_query)
        } else {
            None
        };

        if self.dpr_dirty {
            if let Some(metrics) = measured_metrics {
                self.pending_metrics = Some(metrics);
            }
        }

        self.dpr_dirty = false;

        let resize = self.pending_metrics.take().and_then(|metrics| {
            let scale_changed = self
                .last_emitted_scale_factor
                .is_none_or(|last| !approximately_equal(last, metrics.scale_factor));
            let size_changed = self
                .last_emitted_physical_size
                .is_none_or(|last| last != (metrics.physical_width, metrics.physical_height));

            if !scale_changed && !size_changed {
                return None;
            }

            self.last_emitted_scale_factor = Some(metrics.scale_factor);
            self.last_emitted_physical_size =
                Some((metrics.physical_width, metrics.physical_height));

            if metrics.physical_width == 0 || metrics.physical_height == 0 {
                return Some(ResizeUpdate::Hidden {
                    scale_factor: metrics.scale_factor,
                });
            }

            Some(ResizeUpdate::Visible {
                logical_width: metrics.logical_width,
                logical_height: metrics.logical_height,
                physical_width: metrics.physical_width.min(max_texture_dimension),
                physical_height: metrics.physical_height.min(max_texture_dimension),
                scale_factor: metrics.scale_factor,
            })
        });

        EnvironmentUpdate {
            media_query,
            resize,
        }
    }
}

fn media_query_for_dpr(dpr: f64) -> String {
    format!("(resolution: {dpr}dppx), (-webkit-device-pixel-ratio: {dpr})")
}

fn approximately_equal(left: f32, right: f32) -> bool {
    (left - right).abs() < 0.0001
}

#[cfg(test)]
mod tests {
    use super::{CanvasMetrics, ResizeUpdate, WindowEnvironmentState};

    fn metrics(physical_width: u32, physical_height: u32, scale_factor: f32) -> CanvasMetrics {
        CanvasMetrics {
            physical_width,
            physical_height,
            logical_width: physical_width as f32 / scale_factor,
            logical_height: physical_height as f32 / scale_factor,
            scale_factor,
        }
    }

    #[test]
    fn test_reconcile_emits_visible_resize_for_first_observation() {
        let mut state = WindowEnvironmentState::new(2.0);
        state.queue_resize(metrics(800, 600, 2.0));

        let update = state.reconcile(2.0, None, 16_384);

        assert_eq!(update.media_query, None);
        assert_eq!(
            update.resize,
            Some(ResizeUpdate::Visible {
                logical_width: 400.0,
                logical_height: 300.0,
                physical_width: 800,
                physical_height: 600,
                scale_factor: 2.0,
            })
        );
    }

    #[test]
    fn test_reconcile_uses_measured_metrics_after_dpr_change() {
        let mut state = WindowEnvironmentState::new(1.0);
        state.queue_resize(metrics(400, 300, 1.0));
        let _ = state.reconcile(1.0, None, 16_384);

        state.queue_dpr_change();

        let update = state.reconcile(2.0, Some(metrics(800, 600, 2.0)), 16_384);

        assert_eq!(
            update.media_query,
            Some("(resolution: 2dppx), (-webkit-device-pixel-ratio: 2)".to_string())
        );
        assert_eq!(
            update.resize,
            Some(ResizeUpdate::Visible {
                logical_width: 400.0,
                logical_height: 300.0,
                physical_width: 800,
                physical_height: 600,
                scale_factor: 2.0,
            })
        );
    }

    #[test]
    fn test_reconcile_skips_duplicate_metrics() {
        let mut state = WindowEnvironmentState::new(2.0);
        state.queue_resize(metrics(800, 600, 2.0));
        let _ = state.reconcile(2.0, None, 16_384);

        state.queue_resize(metrics(800, 600, 2.0));
        let update = state.reconcile(2.0, None, 16_384);

        assert_eq!(update.media_query, None);
        assert_eq!(update.resize, None);
    }

    #[test]
    fn test_reconcile_emits_hidden_resize_for_zero_sized_canvas() {
        let mut state = WindowEnvironmentState::new(2.0);
        state.queue_resize(metrics(0, 0, 2.0));

        let update = state.reconcile(2.0, None, 16_384);

        assert_eq!(
            update.resize,
            Some(ResizeUpdate::Hidden { scale_factor: 2.0 })
        );
    }

    #[test]
    fn test_reconcile_clamps_drawable_size() {
        let mut state = WindowEnvironmentState::new(2.0);
        state.queue_resize(metrics(40_000, 30_000, 2.0));

        let update = state.reconcile(2.0, None, 16_384);

        assert_eq!(
            update.resize,
            Some(ResizeUpdate::Visible {
                logical_width: 20_000.0,
                logical_height: 15_000.0,
                physical_width: 16_384,
                physical_height: 16_384,
                scale_factor: 2.0,
            })
        );
    }
}
