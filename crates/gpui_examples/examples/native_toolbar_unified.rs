use gpui::{
    App, Bounds, Context, NativeToolbar, NativeToolbarButton, NativeToolbarClickEvent,
    NativeToolbarControlGroup, NativeToolbarDisplayMode, NativeToolbarGroupControlRepresentation,
    NativeToolbarGroupEvent, NativeToolbarGroupOption, NativeToolbarItem, NativeToolbarSearchEvent,
    NativeToolbarSearchField, Window, WindowAppearance, WindowBounds, WindowOptions,
    WindowToolbarStyle, div, prelude::*, px, rgb, size,
};

struct UnifiedToolbarExample {
    toolbar_installed: bool,
    status: String,
}

impl Render for UnifiedToolbarExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.toolbar_installed {
            window.set_native_toolbar(Some(
                NativeToolbar::new("gpui.native.toolbar.unified")
                    .title("Unified Toolbar")
                    .display_mode(NativeToolbarDisplayMode::IconAndLabel)
                    .item(NativeToolbarItem::ControlGroup(
                        NativeToolbarControlGroup::new(
                            "navigation",
                            vec![
                                NativeToolbarGroupOption::new("Back").icon("chevron.left"),
                                NativeToolbarGroupOption::new("Forward").icon("chevron.right"),
                                NativeToolbarGroupOption::new("Reload").icon("arrow.clockwise"),
                            ],
                        )
                        .control_representation(NativeToolbarGroupControlRepresentation::Expanded)
                        .selected_index(0)
                        .on_select(cx.listener(
                            |this, event: &NativeToolbarGroupEvent, _, cx| {
                                this.status = format!(
                                    "Navigation group changed to segment {}",
                                    event.selected_index
                                );
                                cx.notify();
                            },
                        )),
                    ))
                    .item(NativeToolbarItem::SearchField(
                        NativeToolbarSearchField::new("search")
                            .placeholder("Search unified toolbar")
                            .on_change(cx.listener(
                                |this, event: &NativeToolbarSearchEvent, _, cx| {
                                    this.status = format!("Searching for {}", event.text);
                                    cx.notify();
                                },
                            )),
                    ))
                    .item(NativeToolbarItem::FlexibleSpace)
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new("favorite", "Favorite")
                            .icon("star.fill")
                            .tool_tip("Mark favorite")
                            .on_click(cx.listener(
                                |this, _event: &NativeToolbarClickEvent, _, cx| {
                                    this.status = "Favorited".into();
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
            (rgb(0x16181d), rgb(0xf5f7fa), rgb(0x9aa4b2))
        } else {
            (rgb(0xf5f7fb), rgb(0x16202a), rgb(0x5b6674))
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap_4()
            .p_6()
            .bg(background)
            .text_color(foreground)
            .child(div().text_xl().child("Unified Toolbar"))
            .child(
                div()
                    .text_sm()
                    .text_color(muted)
                    .child("The window title shares the same horizontal band as the toolbar."),
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
                    toolbar_style: WindowToolbarStyle::Unified,
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| UnifiedToolbarExample {
                    toolbar_installed: false,
                    status: "Ready".into(),
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
