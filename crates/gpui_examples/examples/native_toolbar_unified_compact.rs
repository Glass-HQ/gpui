use gpui::{
    App, Bounds, Context, NativeToolbar, NativeToolbarButton, NativeToolbarClickEvent,
    NativeToolbarDisplayMode, NativeToolbarItem, NativeToolbarSearchEvent,
    NativeToolbarSearchField, Window, WindowAppearance, WindowBounds, WindowOptions,
    WindowToolbarStyle, div, prelude::*, px, rgb, size,
};

struct UnifiedCompactToolbarExample {
    toolbar_installed: bool,
    status: String,
}

impl Render for UnifiedCompactToolbarExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.toolbar_installed {
            window.set_native_toolbar(Some(
                NativeToolbar::new("gpui.native.toolbar.unified_compact")
                    .title("Unified Compact Toolbar")
                    .display_mode(NativeToolbarDisplayMode::IconOnly)
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new("sidebar", "Sidebar")
                            .icon("sidebar.left")
                            .on_click(cx.listener(
                                |this, _event: &NativeToolbarClickEvent, _, cx| {
                                    this.status = "Sidebar toggled".into();
                                    cx.notify();
                                },
                            )),
                    ))
                    .item(NativeToolbarItem::FlexibleSpace)
                    .item(NativeToolbarItem::SearchField(
                        NativeToolbarSearchField::new("search")
                            .placeholder("Search compact toolbar")
                            .on_change(cx.listener(
                                |this, event: &NativeToolbarSearchEvent, _, cx| {
                                    this.status = format!("Searching for {}", event.text);
                                    cx.notify();
                                },
                            )),
                    ))
                    .item(NativeToolbarItem::FlexibleSpace)
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new("share", "Share")
                            .icon("square.and.arrow.up")
                            .on_click(cx.listener(
                                |this, _event: &NativeToolbarClickEvent, _, cx| {
                                    this.status = "Share clicked".into();
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
            (rgb(0x14171b), rgb(0xf5f7fa), rgb(0x98a2b3))
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
            .child(div().text_xl().child("Unified Compact Toolbar"))
            .child(
                div()
                    .text_sm()
                    .text_color(muted)
                    .child("This uses the unified layout with tighter margins and denser chrome."),
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
        let bounds = Bounds::centered(None, size(px(820.0), px(480.0)), cx);
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
                cx.new(|_| UnifiedCompactToolbarExample {
                    toolbar_installed: false,
                    status: "Ready".into(),
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
