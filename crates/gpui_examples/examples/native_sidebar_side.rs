use gpui::{
    App, Bounds, Context, Entity, Window, WindowAppearance, WindowBounds, WindowOptions, div,
    native_sidebar, prelude::*, px, rgb, size,
};

struct SidebarPanel;

impl Render for SidebarPanel {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let (bg, fg, muted, border) = match window.appearance() {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => {
                (rgb(0x1f1f21), rgb(0xffffff), rgb(0xa0a0a6), rgb(0x3a3a3c))
            }
            _ => (rgb(0xf7f7f8), rgb(0x1d1d1f), rgb(0x6e6e73), rgb(0xd8d8dc)),
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_2()
            .bg(bg)
            .p_3()
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(fg)
                    .child("Sidebar Surface"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(muted)
                    .child("This is the leading sidebar surface."),
            )
            .child(div().h(px(1.0)).w_full().bg(border))
            .child(div().text_sm().text_color(fg).child("Project"))
            .child(div().text_sm().text_color(fg).child("Agents"))
            .child(div().text_sm().text_color(fg).child("Threads"))
    }
}

struct InspectorPanel;

impl Render for InspectorPanel {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let (bg, fg, muted, border) = match window.appearance() {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => {
                (rgb(0x202124), rgb(0xffffff), rgb(0xa0a0a6), rgb(0x3a3a3c))
            }
            _ => (rgb(0xf8f8fa), rgb(0x1d1d1f), rgb(0x6e6e73), rgb(0xd8d8dc)),
        };

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_2()
            .bg(bg)
            .p_3()
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(fg)
                    .child("Inspector Surface"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(muted)
                    .child("This is the trailing inspector surface."),
            )
            .child(div().h(px(1.0)).w_full().bg(border))
            .child(
                div()
                    .text_sm()
                    .text_color(fg)
                    .child("Selection: native_sidebar_side.rs"),
            )
            .child(div().text_sm().text_color(muted).child("Line endings: LF"))
            .child(div().text_sm().text_color(muted).child("Encoding: UTF-8"))
    }
}

struct NativeSidebarSideExample {
    sidebar_panel: Entity<SidebarPanel>,
    inspector_panel: Entity<InspectorPanel>,
}

impl Render for NativeSidebarSideExample {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let (bg, panel_bg, fg, muted, border) = match window.appearance() {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => (
                rgb(0x1a1a1c),
                rgb(0x242426),
                rgb(0xffffff),
                rgb(0xa0a0a6),
                rgb(0x3a3a3c),
            ),
            _ => (
                rgb(0xf2f2f4),
                rgb(0xffffff),
                rgb(0x1d1d1f),
                rgb(0x6e6e73),
                rgb(0xd8d8dc),
            ),
        };

        div()
            .size_full()
            .bg(bg)
            .text_color(fg)
            .child(
                native_sidebar("sidebar-side-demo", &[""; 0])
                    .sidebar_view(self.sidebar_panel.clone())
                    .sidebar_width(280.0)
                    .min_sidebar_width(200.0)
                    .max_sidebar_width(420.0)
                    .collapsed(false)
                    .inspector_view(self.inspector_panel.clone())
                    .inspector_width(320.0)
                    .min_inspector_width(240.0)
                    .max_inspector_width(420.0)
                    .inspector_collapsed(false)
                    .size_full(),
            )
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .bg(panel_bg)
                    .p_4()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("Native Sidebar + Inspector"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(muted)
                            .child("Use the native toolbar toggles to control the leading sidebar and trailing inspector independently."),
                    )
                    .child(
                        div()
                            .rounded(px(10.0))
                            .border_1()
                            .border_color(border)
                            .p_4()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("What to verify"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(muted)
                                    .child("The standard sidebar toggle should stay on the leading side of the window."),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(muted)
                                    .child("The inspector toggle should appear on the trailing side of the toolbar."),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(muted)
                                    .child("You should be able to leave either pane open by itself, or have both open together."),
                            ),
                    ),
            )
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1280.), px(760.)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_window, cx| {
                let sidebar_panel = cx.new(|_| SidebarPanel);
                let inspector_panel = cx.new(|_| InspectorPanel);
                cx.new(|_| NativeSidebarSideExample {
                    sidebar_panel,
                    inspector_panel,
                })
            },
        )
        .expect("failed to open window");
    });
}
