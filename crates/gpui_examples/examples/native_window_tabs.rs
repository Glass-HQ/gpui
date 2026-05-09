use std::time::Duration;

use gpui::{
    App, Bounds, Context, NativeToolbar, NativeToolbarButton, NativeToolbarClickEvent,
    NativeToolbarDisplayMode, NativeToolbarItem, Window, WindowAppearance, WindowBounds,
    WindowOptions, WindowTabbingMode, WindowTabsOptions, WindowToolbarStyle, div, prelude::*, px,
    rgb, size,
};

const WINDOW_TAB_GROUP: &str = "gpui.native.window.tabs";

struct WindowTabsExample {
    toolbar_installed: bool,
    title: String,
    status: String,
}

impl Render for WindowTabsExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.toolbar_installed {
            window.set_native_toolbar(Some(
                NativeToolbar::new(format!("gpui.native.window.tabs.{}", self.title))
                    .title(self.title.clone())
                    .display_mode(NativeToolbarDisplayMode::IconOnly)
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new("merge", "Merge")
                            .icon("square.stack.3d.up")
                            .tool_tip("Merge all windows in this tab group")
                            .on_click(cx.listener(
                                |this, _event: &NativeToolbarClickEvent, window, cx| {
                                    window.merge_all_windows();
                                    this.status = "Merged all windows".into();
                                    cx.notify();
                                },
                            )),
                    ))
                    .item(NativeToolbarItem::FlexibleSpace)
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new("overview", "Overview")
                            .icon("square.on.square")
                            .tool_tip("Toggle the native window tab overview")
                            .on_click(cx.listener(
                                |this, _event: &NativeToolbarClickEvent, window, cx| {
                                    window.toggle_window_tab_overview();
                                    this.status = "Toggled tab overview".into();
                                    cx.notify();
                                },
                            )),
                    ))
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new("detach", "Detach")
                            .icon("rectangle.split.3x1")
                            .tool_tip("Move the active tab into a new window")
                            .on_click(cx.listener(
                                |this, _event: &NativeToolbarClickEvent, window, cx| {
                                    window.move_tab_to_new_window();
                                    this.status = "Detached current tab".into();
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
            (rgb(0xf8fafc), rgb(0x18212b), rgb(0x667085))
        };
        let tab_count = window.tabbed_windows().map(|tabs| tabs.len()).unwrap_or(1);

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap_4()
            .p_6()
            .bg(background)
            .text_color(foreground)
            .child(div().text_xl().child(self.title.clone()))
            .child(
                div()
                    .text_sm()
                    .text_color(muted)
                    .child("These are ordinary windows merged into one native macOS tab group."),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(muted)
                    .child(format!("Known tabs in group: {}", tab_count)),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(muted)
                    .child(format!("Status: {}", self.status)),
            )
    }
}

fn open_tabbed_window(
    cx: &mut App,
    title: String,
    status: &'static str,
    x_offset: f32,
) -> gpui::WindowHandle<WindowTabsExample> {
    let mut bounds = Bounds::centered(None, size(px(760.0), px(440.0)), cx);
    bounds.origin.x += px(x_offset);

    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(gpui::TitlebarOptions {
                toolbar_style: WindowToolbarStyle::Unified,
                ..Default::default()
            }),
            window_tabs: Some(WindowTabsOptions {
                identifier: WINDOW_TAB_GROUP.into(),
                mode: WindowTabbingMode::Preferred,
            }),
            ..Default::default()
        },
        move |_, cx| {
            let title = title.clone();
            cx.new(|_| WindowTabsExample {
                toolbar_installed: false,
                title: title.clone(),
                status: status.into(),
            })
        },
    )
    .unwrap()
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let primary = open_tabbed_window(cx, "Window Tab A".into(), "Primary tab window", 0.0);
        open_tabbed_window(
            cx,
            "Window Tab B".into(),
            "Secondary window waiting to join the tab group",
            36.0,
        );
        open_tabbed_window(
            cx,
            "Window Tab C".into(),
            "Tertiary window waiting to join the tab group",
            72.0,
        );

        cx.spawn(async move |cx| {
            cx.background_executor()
                .timer(Duration::from_millis(150))
                .await;
            let _ = primary.update(cx, |_, window, _| {
                window.merge_all_windows();
            });
        })
        .detach();

        cx.activate(true);
    });
}
