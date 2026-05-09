use gpui::{
    App, Bounds, Context, NativeToolbar, NativeToolbarDisplayMode, NativeToolbarItem,
    NativeToolbarTab, NativeToolbarTabEvent, NativeToolbarTabs, Window, WindowAppearance,
    WindowBounds, WindowOptions, WindowToolbarStyle, div, prelude::*, px, rgb, size,
};

struct ToolbarTabsExample {
    toolbar_installed: bool,
    selected_tab: usize,
}

impl Render for ToolbarTabsExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.toolbar_installed {
            window.set_native_toolbar(Some(
                NativeToolbar::new("gpui.native.toolbar.tabs")
                    .title("Toolbar Tabs")
                    .display_mode(NativeToolbarDisplayMode::IconAndLabel)
                    .item(NativeToolbarItem::Tabs(
                        NativeToolbarTabs::new(
                            "workspace-tabs",
                            vec![
                                NativeToolbarTab::new("Overview").icon("rectangle.grid.2x2"),
                                NativeToolbarTab::new("Activity").icon("bolt.horizontal"),
                                NativeToolbarTab::new("Settings").icon("gearshape"),
                            ],
                        )
                        .selected_index(0)
                        .on_select(cx.listener(
                            |this, event: &NativeToolbarTabEvent, _, cx| {
                                this.selected_tab = event.selected_index;
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
            (rgb(0x14181d), rgb(0xf4f6f8), rgb(0x9aa4b2))
        } else {
            (rgb(0xf8fafc), rgb(0x17212c), rgb(0x667085))
        };
        let panel = match self.selected_tab {
            0 => "This is the overview panel rendered below the toolbar tabs.",
            1 => "This is the activity panel for the second toolbar tab.",
            _ => "This is the settings panel for the third toolbar tab.",
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap_4()
            .p_6()
            .bg(background)
            .text_color(foreground)
            .child(div().text_xl().child("Toolbar Tabs"))
            .child(
                div()
                    .text_sm()
                    .text_color(muted)
                    .child("This demo mirrors the Swift lab's Overview / Activity / Settings toolbar tabs naming."),
            )
            .child(div().text_base().child(panel))
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(860.0), px(480.0)), cx);
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
                cx.new(|_| ToolbarTabsExample {
                    toolbar_installed: false,
                    selected_tab: 0,
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
