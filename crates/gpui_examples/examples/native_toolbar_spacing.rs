use gpui::{
    App, Bounds, Context, NativeToolbar, NativeToolbarButton, NativeToolbarClickEvent,
    NativeToolbarDisplayMode, NativeToolbarItem, NativeToolbarLabel, Window, WindowAppearance,
    WindowBounds, WindowOptions, WindowToolbarStyle, div, prelude::*, px, rgb, size,
};

struct ToolbarSpacingExample {
    toolbar_installed: bool,
    status: String,
}

impl Render for ToolbarSpacingExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.toolbar_installed {
            window.set_native_toolbar(Some(
                NativeToolbar::new("gpui.native.toolbar.spacing")
                    .title("Toolbar Spacing")
                    .display_mode(NativeToolbarDisplayMode::IconAndLabel)
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new("left", "Sidebar")
                            .icon("sidebar.left")
                            .on_click(cx.listener(
                                |this, _event: &NativeToolbarClickEvent, _, cx| {
                                    this.status = "Left cluster clicked".into();
                                    cx.notify();
                                },
                            )),
                    ))
                    .item(NativeToolbarItem::Space)
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new("filters", "Filters")
                            .icon("line.3.horizontal.decrease.circle")
                            .on_click(cx.listener(
                                |this, _event: &NativeToolbarClickEvent, _, cx| {
                                    this.status = "Fixed space keeps this visually attached".into();
                                    cx.notify();
                                },
                            )),
                    ))
                    .item(NativeToolbarItem::FlexibleSpace)
                    .item(NativeToolbarItem::Label(NativeToolbarLabel::new(
                        "center",
                        "Centered by flexible spacers",
                    )))
                    .item(NativeToolbarItem::FlexibleSpace)
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new("new", "New")
                            .icon("plus")
                            .on_click(cx.listener(
                                |this, _event: &NativeToolbarClickEvent, _, cx| {
                                    this.status = "Right cluster clicked".into();
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
            (rgb(0x15181d), rgb(0xf4f6f8), rgb(0x9aa4b2))
        } else {
            (rgb(0xf7f8fb), rgb(0x18212b), rgb(0x667085))
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap_4()
            .p_6()
            .bg(background)
            .text_color(foreground)
            .child(div().text_xl().child("Toolbar Spacing"))
            .child(
                div()
                    .text_sm()
                    .text_color(muted)
                    .child("Use fixed spaces to keep related controls together and flexible spaces to create true separation."),
            )
            .child(div().text_sm().text_color(muted).child(format!(
                "Status: {}",
                self.status
            )))
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(900.0), px(440.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    toolbar_style: WindowToolbarStyle::Unified,
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| ToolbarSpacingExample {
                    toolbar_installed: false,
                    status: "Ready".into(),
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
