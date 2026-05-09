use gpui::{
    App, Bounds, Context, NativePanel, NativePanelAnchor, NativePanelLevel, NativePanelStyle,
    NativeToolbar, NativeToolbarButton, NativeToolbarClickEvent, NativeToolbarDisplayMode,
    NativeToolbarItem, NativeToolbarSizeMode, SharedString, WeakEntity, Window, WindowAppearance,
    WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
};

#[derive(Clone, Copy)]
enum ToolbarOverlayKind {
    Project,
    Branches,
    LanguageServers,
}

impl ToolbarOverlayKind {
    fn item_id(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Branches => "branches",
            Self::LanguageServers => "lsp",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Project => "Project",
            Self::Branches => "Branches",
            Self::LanguageServers => "LSP",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Project => "folder",
            Self::Branches => "arrow.triangle.branch",
            Self::LanguageServers => "bolt",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Project => "Recent Projects",
            Self::Branches => "Git Branches",
            Self::LanguageServers => "Language Servers",
        }
    }

    fn subtitle(self) -> &'static str {
        match self {
            Self::Project => "Hosted GPUI content anchored below a native toolbar item.",
            Self::Branches => "This uses a transient borderless panel, not an NSPopover shell.",
            Self::LanguageServers => {
                "The overlay card is rendered by GPUI and owns its own styling."
            }
        }
    }

    fn size(self) -> (f64, f64) {
        match self {
            Self::Project => (360.0, 240.0),
            Self::Branches => (380.0, 260.0),
            Self::LanguageServers => (340.0, 220.0),
        }
    }

    fn rows(self) -> [(&'static str, &'static str); 3] {
        match self {
            Self::Project => [
                ("Open Glass", "~/Developer/Glass-HQ/Glass"),
                ("Open GPUI", "~/Developer/Glass-HQ/gpui"),
                ("Open Scratch", "~/Developer/Scratch"),
            ],
            Self::Branches => [
                ("main", "Stable integration branch"),
                (
                    "feature/toolbar-overlays",
                    "Experimental hosted-overlay work",
                ),
                ("fix/sidebar-host", "Left-sidebar host cleanup"),
            ],
            Self::LanguageServers => [
                ("rust-analyzer", "Healthy · 124 MB"),
                ("tsserver", "Healthy · 88 MB"),
                ("gopls", "Starting · waiting for workspace"),
            ],
        }
    }
}

struct ToolbarHostedOverlayExample {
    toolbar_installed: bool,
    status_message: String,
    overlay_card: gpui::Entity<ToolbarOverlayCard>,
}

impl ToolbarHostedOverlayExample {
    fn show_overlay(
        &mut self,
        kind: ToolbarOverlayKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let owner = cx.entity().downgrade();
        self.overlay_card.update(cx, |card, cx| {
            card.title = kind.title().to_string();
            card.subtitle = kind.subtitle().to_string();
            card.rows = kind
                .rows()
                .into_iter()
                .map(|(label, detail)| OverlayRow {
                    label: label.to_string(),
                    detail: detail.to_string(),
                })
                .collect();
            card.owner = Some(owner);
            cx.notify();
        });

        let (width, height) = kind.size();
        let weak_example = cx.entity().downgrade();
        window.dismiss_native_panel();
        window.show_native_panel(
            NativePanel::new(width, height)
                .style(NativePanelStyle::Borderless)
                .level(NativePanelLevel::PopUpMenu)
                .transient(true)
                .corner_radius(12.0)
                .on_close(move |_, _, cx| {
                    weak_example
                        .update(cx, |example, cx| {
                            example.status_message = "Overlay dismissed".to_string();
                            cx.notify();
                        })
                        .ok();
                })
                .content_view(self.overlay_card.clone()),
            NativePanelAnchor::ToolbarItem(kind.item_id().into()),
        );

        self.status_message = format!("Opened {} overlay", kind.label());
        cx.notify();
    }
}

impl Render for ToolbarHostedOverlayExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.toolbar_installed {
            let project_kind = ToolbarOverlayKind::Project;
            let branch_kind = ToolbarOverlayKind::Branches;
            let lsp_kind = ToolbarOverlayKind::LanguageServers;

            window.set_native_toolbar(Some(
                NativeToolbar::new("gpui.toolbar.gpui.overlay.example")
                    .title("Toolbar GPUI Overlay Demo")
                    .display_mode(NativeToolbarDisplayMode::IconAndLabel)
                    .size_mode(NativeToolbarSizeMode::Regular)
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new(project_kind.item_id(), project_kind.label())
                            .icon(project_kind.icon())
                            .tool_tip("Open a hosted GPUI project overlay")
                            .on_click(cx.listener(
                                move |this, _: &NativeToolbarClickEvent, window, cx| {
                                    this.show_overlay(project_kind, window, cx);
                                },
                            )),
                    ))
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new(branch_kind.item_id(), branch_kind.label())
                            .icon(branch_kind.icon())
                            .tool_tip("Open a hosted GPUI branch overlay")
                            .on_click(cx.listener(
                                move |this, _: &NativeToolbarClickEvent, window, cx| {
                                    this.show_overlay(branch_kind, window, cx);
                                },
                            )),
                    ))
                    .item(NativeToolbarItem::FlexibleSpace)
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new(lsp_kind.item_id(), lsp_kind.label())
                            .icon(lsp_kind.icon())
                            .tool_tip("Open a hosted GPUI language-server overlay")
                            .on_click(cx.listener(
                                move |this, _: &NativeToolbarClickEvent, window, cx| {
                                    this.show_overlay(lsp_kind, window, cx);
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
            (rgb(0x17191d), rgb(0xf3f4f6), rgb(0x98a2b3))
        } else {
            (rgb(0xf4f5f7), rgb(0x14181f), rgb(0x667085))
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap_4()
            .p_6()
            .bg(background)
            .text_color(foreground)
            .child(div().text_xl().child("Native Toolbar GPUI Overlays"))
            .child(
                div()
                    .max_w(px(560.0))
                    .text_sm()
                    .text_color(muted)
                    .child(
                        "This example anchors a transient borderless panel to native toolbar items and renders the overlay body with GPUI content.",
                    ),
            )
            .child(
                div()
                    .rounded(px(10.0))
                    .border_1()
                    .border_color(muted)
                    .px_3()
                    .py_2()
                    .child(self.status_message.clone()),
            )
    }
}

#[derive(Clone)]
struct OverlayRow {
    label: String,
    detail: String,
}

struct ToolbarOverlayCard {
    title: String,
    subtitle: String,
    rows: Vec<OverlayRow>,
    owner: Option<WeakEntity<ToolbarHostedOverlayExample>>,
}

impl ToolbarOverlayCard {
    fn render_row(
        &self,
        row: OverlayRow,
        hover: gpui::Rgba,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let label = row.label;
        let detail = row.detail;
        let selected_label = label.clone();
        let owner = self.owner.clone();

        div()
            .id(SharedString::from(format!("row-{label}")))
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .rounded(px(8.0))
            .px_3()
            .py_2()
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .on_click(cx.listener(move |_, _, window, cx| {
                window.dismiss_native_panel();
                if let Some(owner) = owner.as_ref() {
                    owner
                        .update(cx, |example, cx| {
                            example.status_message = format!("Selected {selected_label}");
                            cx.notify();
                        })
                        .ok();
                }
            }))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .overflow_hidden()
                    .child(div().text_sm().truncate().child(label))
                    .child(div().text_xs().truncate().child(detail)),
            )
            .child(div().text_xs().child("GPUI"))
    }
}

impl Render for ToolbarOverlayCard {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_dark = matches!(
            window.appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        let (background, border, foreground, muted, hover) = if is_dark {
            (
                rgb(0x20242c),
                rgb(0x343943),
                rgb(0xf3f4f6),
                rgb(0xa0aec0),
                rgb(0x2b313a),
            )
        } else {
            (
                rgb(0xffffff),
                rgb(0xd0d5dd),
                rgb(0x14181f),
                rgb(0x667085),
                rgb(0xf2f4f7),
            )
        };

        let mut rows = div().flex().flex_col().gap_1();
        for row in self.rows.iter().cloned() {
            rows = rows.child(self.render_row(row, hover, cx));
        }

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap_2()
            .rounded(px(12.0))
            .border_1()
            .border_color(border)
            .bg(background)
            .p_3()
            .text_color(foreground)
            .child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(self.title.clone()),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .child(self.subtitle.clone()),
            )
            .child(div().h(px(1.0)).bg(border))
            .child(rows)
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(700.0), px(420.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_window, cx| {
                cx.new(|cx| ToolbarHostedOverlayExample {
                    toolbar_installed: false,
                    status_message: "Choose a toolbar item to open a hosted GPUI overlay"
                        .to_string(),
                    overlay_card: cx.new(|_| ToolbarOverlayCard {
                        title: String::new(),
                        subtitle: String::new(),
                        rows: Vec::new(),
                        owner: None,
                    }),
                })
            },
        )
        .unwrap();
    });
}
