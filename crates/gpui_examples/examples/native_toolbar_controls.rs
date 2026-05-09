use gpui::{
    App, Bounds, Context, NativeColor, NativeToolbar, NativeToolbarBadge, NativeToolbarButton,
    NativeToolbarClickEvent, NativeToolbarControlGroup, NativeToolbarDisplayMode,
    NativeToolbarGroupControlRepresentation, NativeToolbarGroupEvent, NativeToolbarGroupOption,
    NativeToolbarItem, NativeToolbarItemStyle, Window, WindowAppearance, WindowBounds,
    WindowOptions, WindowToolbarStyle, div, prelude::*, px, rgb, size,
};

struct ToolbarControlsExample {
    toolbar_installed: bool,
    status: String,
}

impl Render for ToolbarControlsExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.toolbar_installed {
            window.set_native_toolbar(Some(
                NativeToolbar::new("gpui.native.toolbar.controls")
                    .title("Toolbar Controls")
                    .display_mode(NativeToolbarDisplayMode::IconAndLabel)
                    .item(NativeToolbarItem::ControlGroup(
                        NativeToolbarControlGroup::new(
                            "view-mode",
                            vec![
                                NativeToolbarGroupOption::new("List"),
                                NativeToolbarGroupOption::new("Board"),
                                NativeToolbarGroupOption::new("Timeline"),
                            ],
                        )
                        .control_representation(NativeToolbarGroupControlRepresentation::Expanded)
                        .selected_index(0)
                        .on_select(cx.listener(
                            |this, event: &NativeToolbarGroupEvent, _, cx| {
                                this.status =
                                    format!("Group changed to index {}", event.selected_index);
                                cx.notify();
                            },
                        )),
                    ))
                    .item(NativeToolbarItem::FlexibleSpace)
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new("compose", "Compose")
                            .icon("square.and.pencil")
                            .style(NativeToolbarItemStyle::Prominent)
                            .background_tint(NativeColor::Blue)
                            .on_click(cx.listener(
                                |this, _event: &NativeToolbarClickEvent, _, cx| {
                                    this.status = "Prominent tinted action clicked".into();
                                    cx.notify();
                                },
                            )),
                    ))
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new("notifications", "Notifications")
                            .icon("bell")
                            .badge(NativeToolbarBadge::Count(5))
                            .on_click(cx.listener(
                                |this, _event: &NativeToolbarClickEvent, _, cx| {
                                    this.status = "Badged action clicked".into();
                                    cx.notify();
                                },
                            )),
                    ))
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new("sync", "Sync")
                            .icon("arrow.triangle.2.circlepath")
                            .badge(NativeToolbarBadge::Indicator)
                            .on_click(cx.listener(
                                |this, _event: &NativeToolbarClickEvent, _, cx| {
                                    this.status = "Indicator badge clicked".into();
                                    cx.notify();
                                },
                            )),
                    )),
            ));
            self.toolbar_installed = true;
        }

        let is_dark = matches!(
            window.appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        let (background, foreground, muted) = if is_dark {
            (rgb(0x14171c), rgb(0xf4f6f8), rgb(0x98a2b3))
        } else {
            (rgb(0xf8fafc), rgb(0x18212b), rgb(0x667085))
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap_4()
            .p_6()
            .bg(background)
            .text_color(foreground)
            .child(div().text_xl().child("Toolbar Controls"))
            .child(
                div()
                    .text_sm()
                    .text_color(muted)
                    .child("This example focuses on grouped controls, prominent tinted actions, and toolbar badges."),
            )
            .child(div().text_sm().text_color(muted).child(format!(
                "Status: {}",
                self.status
            )))
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(860.0), px(460.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    toolbar_style: WindowToolbarStyle::UnifiedCompact,
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| ToolbarControlsExample {
                    toolbar_installed: false,
                    status: "Ready".into(),
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
