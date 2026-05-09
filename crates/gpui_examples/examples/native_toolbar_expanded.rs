use gpui::{
    App, Bounds, Context, NativeToolbar, NativeToolbarButton, NativeToolbarClickEvent,
    NativeToolbarDisplayMode, NativeToolbarItem, NativeToolbarSearchEvent,
    NativeToolbarSearchField, Window, WindowAppearance, WindowBounds, WindowOptions,
    WindowToolbarStyle, div, prelude::*, px, rgb, size,
};

struct ExpandedToolbarExample {
    toolbar_installed: bool,
    status: String,
}

impl Render for ExpandedToolbarExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.toolbar_installed {
            window.set_native_toolbar(Some(
                NativeToolbar::new("gpui.native.toolbar.expanded")
                    .title("Expanded Toolbar")
                    .display_mode(NativeToolbarDisplayMode::IconAndLabel)
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new("refresh", "Refresh")
                            .icon("arrow.clockwise")
                            .on_click(cx.listener(
                                |this, _event: &NativeToolbarClickEvent, _, cx| {
                                    this.status = "Refresh clicked".into();
                                    cx.notify();
                                },
                            )),
                    ))
                    .item(NativeToolbarItem::FlexibleSpace)
                    .item(NativeToolbarItem::SearchField(
                        NativeToolbarSearchField::new("search")
                            .placeholder("Search expanded toolbar")
                            .on_change(cx.listener(
                                |this, event: &NativeToolbarSearchEvent, _, cx| {
                                    this.status = format!("Searching for {}", event.text);
                                    cx.notify();
                                },
                            )),
                    ))
                    .item(NativeToolbarItem::FlexibleSpace)
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new("download", "Download")
                            .icon("arrow.down.circle")
                            .on_click(cx.listener(
                                |this, _event: &NativeToolbarClickEvent, _, cx| {
                                    this.status = "Download clicked".into();
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
            (rgb(0x15181c), rgb(0xf4f6f8), rgb(0x9aa4b2))
        } else {
            (rgb(0xf6f8fb), rgb(0x17202c), rgb(0x5f6c7b))
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap_4()
            .p_6()
            .bg(background)
            .text_color(foreground)
            .child(div().text_xl().child("Expanded Toolbar"))
            .child(
                div().text_sm().text_color(muted).child(
                    "Expanded style drops the toolbar below the title instead of merging them.",
                ),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(muted)
                    .child(format!("Status: {}", self.status)),
            )
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(840.0), px(500.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    toolbar_style: WindowToolbarStyle::Expanded,
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| ExpandedToolbarExample {
                    toolbar_installed: false,
                    status: "Ready".into(),
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
