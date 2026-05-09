use gpui::{
    App, Bounds, Context, NativeColor, NativeToolbar, NativeToolbarButton, NativeToolbarClickEvent,
    NativeToolbarDisplayMode, NativeToolbarItem, NativeToolbarItemStyle, Window, WindowAppearance,
    WindowBounds, WindowOptions, WindowToolbarStyle, div, prelude::*, px, rgb, size,
};

struct PreferenceToolbarExample {
    installed_selected_index: Option<usize>,
    selected_index: usize,
}

impl Render for PreferenceToolbarExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.installed_selected_index != Some(self.selected_index) {
            window.set_native_toolbar(Some(
                NativeToolbar::new("gpui.native.toolbar.preference")
                    .title("Preference Toolbar")
                    .display_mode(NativeToolbarDisplayMode::IconAndLabel)
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new("general", "General")
                            .icon("gearshape")
                            .style(if self.selected_index == 0 {
                                NativeToolbarItemStyle::Prominent
                            } else {
                                NativeToolbarItemStyle::Plain
                            })
                            .background_tint(NativeColor::Blue)
                            .on_click(cx.listener(
                                |this, _event: &NativeToolbarClickEvent, _, cx| {
                                    this.selected_index = 0;
                                    cx.notify();
                                },
                            )),
                    ))
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new("accounts", "Accounts")
                            .icon("person.crop.circle")
                            .style(if self.selected_index == 1 {
                                NativeToolbarItemStyle::Prominent
                            } else {
                                NativeToolbarItemStyle::Plain
                            })
                            .background_tint(NativeColor::Blue)
                            .on_click(cx.listener(
                                |this, _event: &NativeToolbarClickEvent, _, cx| {
                                    this.selected_index = 1;
                                    cx.notify();
                                },
                            )),
                    ))
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new("updates", "Updates")
                            .icon("arrow.triangle.2.circlepath")
                            .style(if self.selected_index == 2 {
                                NativeToolbarItemStyle::Prominent
                            } else {
                                NativeToolbarItemStyle::Plain
                            })
                            .background_tint(NativeColor::Blue)
                            .on_click(cx.listener(
                                |this, _event: &NativeToolbarClickEvent, _, cx| {
                                    this.selected_index = 2;
                                    cx.notify();
                                },
                            )),
                    ))
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new("advanced", "Advanced")
                            .icon("slider.horizontal.3")
                            .style(if self.selected_index == 3 {
                                NativeToolbarItemStyle::Prominent
                            } else {
                                NativeToolbarItemStyle::Plain
                            })
                            .background_tint(NativeColor::Blue)
                            .on_click(cx.listener(
                                |this, _event: &NativeToolbarClickEvent, _, cx| {
                                    this.selected_index = 3;
                                    cx.notify();
                                },
                            )),
                    )),
            ));
            self.installed_selected_index = Some(self.selected_index);
        }

        let is_dark = matches!(
            window.appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        let (background, foreground, muted) = if is_dark {
            (rgb(0x16181d), rgb(0xf4f6f8), rgb(0x98a2b3))
        } else {
            (rgb(0xf9fafb), rgb(0x18212b), rgb(0x667085))
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap_4()
            .p_6()
            .bg(background)
            .text_color(foreground)
            .child(div().text_xl().child("Preference Toolbar"))
            .child(
                div()
                    .text_sm()
                    .text_color(muted)
                    .child("Preference style works best for top-level settings sections."),
            )
            .child(div().text_sm().text_color(muted).child(format!(
                "Selected section: {}",
                match self.selected_index {
                    0 => "General",
                    1 => "Accounts",
                    2 => "Updates",
                    _ => "Advanced",
                }
            )))
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(780.0), px(440.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    toolbar_style: WindowToolbarStyle::Preference,
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| PreferenceToolbarExample {
                    installed_selected_index: None,
                    selected_index: 0,
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
