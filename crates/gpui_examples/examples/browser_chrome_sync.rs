use gpui::{
    App, Bounds, Context, NativeToolbar, NativeToolbarButton, NativeToolbarClickEvent,
    NativeToolbarDisplayMode, NativeToolbarItem, NativeToolbarSearchEvent,
    NativeToolbarSearchField, NativeToolbarTab, NativeToolbarTabEvent, NativeToolbarTabs,
    TitlebarOptions, Window, WindowBounds, WindowOptions, WindowToolbarStyle, div, native_button,
    prelude::*, px, rgb, size,
};

struct PagePreset {
    name: &'static str,
    url: &'static str,
    header_title: &'static str,
    header_detail: &'static str,
    banner_label: &'static str,
    header_color: u32,
    header_text: u32,
    banner_color: u32,
    banner_text: u32,
    body_background: u32,
    panel_background: u32,
    panel_border: u32,
    body_text: u32,
    muted_text: u32,
}

const PAGE_PRESETS: [PagePreset; 3] = [
    PagePreset {
        name: "Docs",
        url: "https://docs.example.com/design/browser-chrome",
        header_title: "Design system notes",
        header_detail: "A calm documentation page with a slate-blue masthead.",
        banner_label: "Release banner: New design tokens are rolling out",
        header_color: 0x1d4e89,
        header_text: 0xf4f8ff,
        banner_color: 0xf0b429,
        banner_text: 0x2f2100,
        body_background: 0xf4f7fb,
        panel_background: 0xffffff,
        panel_border: 0xd6dee8,
        body_text: 0x18212b,
        muted_text: 0x607080,
    },
    PagePreset {
        name: "Shop",
        url: "https://store.example.com/spring-drop",
        header_title: "Spring collection",
        header_detail: "A commerce page with a warm product rail and brighter promo banner.",
        banner_label: "Promo banner: Free shipping on orders over $50",
        header_color: 0x0f766e,
        header_text: 0xf0fdfa,
        banner_color: 0xfb7185,
        banner_text: 0x3b0614,
        body_background: 0xfff8f8,
        panel_background: 0xffffff,
        panel_border: 0xf2d4d9,
        body_text: 0x2a1519,
        muted_text: 0x77555d,
    },
    PagePreset {
        name: "News",
        url: "https://news.example.com/front-page",
        header_title: "Morning briefing",
        header_detail: "A news front page where the top stripe changes with breaking alerts.",
        banner_label: "Breaking banner: Markets open sharply higher",
        header_color: 0x7c3aed,
        header_text: 0xf7f3ff,
        banner_color: 0x22c55e,
        banner_text: 0x052e16,
        body_background: 0xf6f2ff,
        panel_background: 0xffffff,
        panel_border: 0xe4dafb,
        body_text: 0x221433,
        muted_text: 0x6f5a89,
    },
];

struct BrowserChromeSyncExample {
    toolbar_installed: bool,
    selected_page: usize,
    show_banner: bool,
    query: String,
    status: String,
}

impl BrowserChromeSyncExample {
    fn current_page(&self) -> &'static PagePreset {
        &PAGE_PRESETS[self.selected_page]
    }

    fn chrome_source_label(&self) -> &'static str {
        if self.show_banner { "banner" } else { "header" }
    }
}

impl Render for BrowserChromeSyncExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.toolbar_installed {
            window.set_native_toolbar(Some(
                NativeToolbar::new("gpui.browser.chrome.sync")
                    .title("Browser Chrome Sync")
                    .display_mode(NativeToolbarDisplayMode::IconOnly)
                    .shows_baseline_separator(false)
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new("back", "Back")
                            .icon("chevron.left")
                            .tool_tip("Back")
                            .on_click(cx.listener(
                                |this, _event: &NativeToolbarClickEvent, _, cx| {
                                    this.status = "Back clicked".into();
                                    cx.notify();
                                },
                            )),
                    ))
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new("forward", "Forward")
                            .icon("chevron.right")
                            .tool_tip("Forward")
                            .on_click(cx.listener(
                                |this, _event: &NativeToolbarClickEvent, _, cx| {
                                    this.status = "Forward clicked".into();
                                    cx.notify();
                                },
                            )),
                    ))
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new("reload", "Reload")
                            .icon("arrow.clockwise")
                            .tool_tip("Reload")
                            .on_click(cx.listener(
                                |this, _event: &NativeToolbarClickEvent, _, cx| {
                                    this.status = "Reload clicked".into();
                                    cx.notify();
                                },
                            )),
                    ))
                    .item(NativeToolbarItem::Tabs(
                        NativeToolbarTabs::new(
                            "browser-tabs",
                            PAGE_PRESETS
                                .iter()
                                .map(|page| NativeToolbarTab::new(page.name))
                                .collect(),
                        )
                        .selected_index(self.selected_page)
                        .on_select(cx.listener(
                            |this, event: &NativeToolbarTabEvent, _, cx| {
                                this.selected_page = event.selected_index;
                                this.status = format!(
                                    "Switched to the {} preset",
                                    PAGE_PRESETS[event.selected_index].name
                                );
                                cx.notify();
                            },
                        )),
                    ))
                    .item(NativeToolbarItem::FlexibleSpace)
                    .item(NativeToolbarItem::SearchField(
                        NativeToolbarSearchField::new("browser-search")
                            .placeholder("Search or enter website name")
                            .min_width(px(220.0))
                            .max_width(px(340.0))
                            .preferred_width_for_search_field(px(360.0))
                            .on_change(cx.listener(
                                |this, event: &NativeToolbarSearchEvent, _, cx| {
                                    this.query = event.text.clone();
                                    cx.notify();
                                },
                            ))
                            .on_submit(cx.listener(
                                |this, event: &NativeToolbarSearchEvent, _, cx| {
                                    this.query = event.text.clone();
                                    this.status = format!("Search submitted for {}", event.text);
                                    cx.notify();
                                },
                            )),
                    ))
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new("download", "Download")
                            .icon("arrow.down.circle")
                            .tool_tip("Download")
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

        let page = self.current_page();
        let titlebar_height = window.titlebar_height();
        let chrome_source = self.chrome_source_label();
        let header_color = rgb(page.header_color);
        let header_text = rgb(page.header_text);
        let banner_color = rgb(page.banner_color);
        let banner_text = rgb(page.banner_text);
        let body_background = rgb(page.body_background);
        let panel_background = rgb(page.panel_background);
        let panel_border = rgb(page.panel_border);
        let body_text = rgb(page.body_text);
        let muted_text = rgb(page.muted_text);

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(body_background)
            .text_color(body_text)
            .when(self.show_banner, |root| {
                root.child(
                    div()
                        .w_full()
                        .bg(banner_color)
                        .text_color(banner_text)
                        .pt(titlebar_height + px(10.0))
                        .pb(px(12.0))
                        .px_6()
                        .flex()
                        .justify_between()
                        .items_center()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(div().text_xs().child("Topmost page UI"))
                                .child(div().text_base().child(page.banner_label)),
                        )
                        .child(
                            div()
                                .text_sm()
                                .child("Because this banner sits at y=0, the toolbar reads as this color."),
                        ),
                )
            })
            .child(
                div()
                    .w_full()
                    .bg(header_color)
                    .text_color(header_text)
                    .pt(if self.show_banner {
                        px(18.0)
                    } else {
                        titlebar_height + px(18.0)
                    })
                    .pb(px(26.0))
                    .px_6()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(div().text_sm().child(page.url))
                    .child(div().text_xl().child(page.header_title))
                    .child(div().text_sm().child(page.header_detail))
                    .child(
                        div().text_sm().child(format!(
                            "The native titlebar is transparent, so the toolbar borrows the {} color without any pixel sampling.",
                            chrome_source
                        )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .gap_5()
                    .p_6()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(
                                native_button(
                                    "toggle-banner",
                                    if self.show_banner {
                                        "Hide top banner"
                                    } else {
                                        "Show top banner"
                                    },
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.show_banner = !this.show_banner;
                                    this.status = if this.show_banner {
                                        "Banner enabled; top chrome should follow the banner".into()
                                    } else {
                                        "Banner hidden; top chrome should fall back to the header".into()
                                    };
                                    cx.notify();
                                })),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(muted_text)
                                    .child(format!(
                                        "Active preset: {}. Chrome source: {}.",
                                        page.name, chrome_source
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .rounded(px(18.0))
                            .border_1()
                            .border_color(panel_border)
                            .bg(panel_background)
                            .p_5()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(div().text_lg().child("What this proves"))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(muted_text)
                                    .child("GPUI does not need a dedicated titlebar background API to get the browser effect. The page can extend behind a transparent native titlebar, and the browser chrome will visually match whatever is actually at the top of the page."),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(muted_text)
                                    .child("Switch tabs from the native toolbar, then toggle the banner. If the topmost visible band changes, the titlebar should change with it."),
                            ),
                    )
                    .child(
                        div()
                            .rounded(px(18.0))
                            .border_1()
                            .border_color(panel_border)
                            .bg(panel_background)
                            .p_5()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(div().text_lg().child("Current state"))
                            .child(div().text_sm().child(format!("Status: {}", self.status)))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(muted_text)
                                    .child(format!(
                                        "Toolbar query: {}",
                                        if self.query.is_empty() {
                                            "<empty>".to_string()
                                        } else {
                                            self.query.clone()
                                        }
                                    )),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(muted_text)
                                    .child(format!(
                                        "Header color: #{:06x} · Banner color: #{:06x}",
                                        page.header_color, page.banner_color
                                    )),
                            ),
                    ),
            )
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1220.0), px(780.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    appears_transparent: true,
                    toolbar_style: WindowToolbarStyle::Unified,
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| BrowserChromeSyncExample {
                    toolbar_installed: false,
                    selected_page: 0,
                    show_banner: false,
                    query: String::new(),
                    status: "Ready".into(),
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
