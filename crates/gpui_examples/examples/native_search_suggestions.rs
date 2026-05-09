/// Native Search Suggestions Example
///
/// Demonstrates a browser-style search experience with:
/// - `NSSearchToolbarItem` in the title bar
/// - `NSSearchField` in content for a new-tab-style search box
/// - a native search suggestion menu anchored to either search field
/// - keyboard navigation that stays on the search field instead of moving focus
use gpui::{
    App, Bounds, Context, Entity, NativeSearchFieldTarget, NativeSearchSuggestionMenu,
    NativeToolbar, NativeToolbarButton, NativeToolbarClickEvent, NativeToolbarDisplayMode,
    NativeToolbarItem, NativeToolbarSearchEvent, NativeToolbarSearchField, SearchChangeEvent,
    SearchSubmitEvent, StatefulInteractiveElement, Styled, WeakEntity, Window, WindowAppearance,
    WindowBounds, WindowOptions, div, native_search_field, prelude::*, px, rgb, rgba, size,
};

const TOOLBAR_SEARCH_ID: &str = "search.demo.toolbar";
const CONTENT_SEARCH_ID: &str = "search.demo.content";
const MENU_WIDTH: f64 = 460.0;
const MENU_MIN_HEIGHT: f64 = 148.0;
const MENU_MAX_HEIGHT: f64 = 348.0;
const ROW_HEIGHT: f64 = 48.0;
const HEADER_HEIGHT: f64 = 36.0;
const MENU_PADDING: f64 = 12.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SearchAnchor {
    Toolbar,
    Content,
}

impl SearchAnchor {
    fn title(self) -> &'static str {
        match self {
            SearchAnchor::Toolbar => "Toolbar Search",
            SearchAnchor::Content => "New Tab Search",
        }
    }

    fn target(self) -> NativeSearchFieldTarget {
        match self {
            SearchAnchor::Toolbar => NativeSearchFieldTarget::ToolbarItem(TOOLBAR_SEARCH_ID.into()),
            SearchAnchor::Content => {
                NativeSearchFieldTarget::ContentElement(CONTENT_SEARCH_ID.into())
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SuggestionKind {
    Search,
    TopHit,
    Bookmark,
    History,
}

impl SuggestionKind {
    fn badge(self) -> &'static str {
        match self {
            SuggestionKind::Search => "Search",
            SuggestionKind::TopHit => "Top Hit",
            SuggestionKind::Bookmark => "Bookmark",
            SuggestionKind::History => "History",
        }
    }
}

#[derive(Clone, Debug)]
struct SuggestionRow {
    kind: SuggestionKind,
    title: String,
    detail: String,
    url: String,
}

#[derive(Clone, Copy)]
struct DemoSuggestion {
    kind: SuggestionKind,
    title: &'static str,
    detail: &'static str,
    url: &'static str,
}

const SUGGESTIONS: &[DemoSuggestion] = &[
    DemoSuggestion {
        kind: SuggestionKind::TopHit,
        title: "Glass Browser",
        detail: "glass.dev",
        url: "https://glass.dev",
    },
    DemoSuggestion {
        kind: SuggestionKind::TopHit,
        title: "GPUI Documentation",
        detail: "gpui.rs",
        url: "https://gpui.rs",
    },
    DemoSuggestion {
        kind: SuggestionKind::Bookmark,
        title: "Apple Developer",
        detail: "developer.apple.com",
        url: "https://developer.apple.com",
    },
    DemoSuggestion {
        kind: SuggestionKind::Bookmark,
        title: "Rust Standard Library",
        detail: "doc.rust-lang.org/std",
        url: "https://doc.rust-lang.org/std/",
    },
    DemoSuggestion {
        kind: SuggestionKind::History,
        title: "Liquid Glass Adoption Guide",
        detail: "developer.apple.com/documentation/technologyoverviews/adopting-liquid-glass",
        url: "https://developer.apple.com/documentation/technologyoverviews/adopting-liquid-glass",
    },
    DemoSuggestion {
        kind: SuggestionKind::History,
        title: "NSSearchToolbarItem",
        detail: "developer.apple.com/documentation/appkit/nssearchtoolbaritem",
        url: "https://developer.apple.com/documentation/appkit/nssearchtoolbaritem",
    },
    DemoSuggestion {
        kind: SuggestionKind::History,
        title: "NSPopover",
        detail: "developer.apple.com/documentation/appkit/nspopover",
        url: "https://developer.apple.com/documentation/appkit/nspopover",
    },
    DemoSuggestion {
        kind: SuggestionKind::History,
        title: "Glass Repo",
        detail: "github.com/Glass-HQ/Glass",
        url: "https://github.com/Glass-HQ/Glass",
    },
];

struct SearchResultsView {
    controller: WeakEntity<SearchSuggestionsExample>,
    anchor: SearchAnchor,
    rows: Vec<SuggestionRow>,
    selected_index: Option<usize>,
}

impl SearchResultsView {
    fn new(controller: WeakEntity<SearchSuggestionsExample>) -> Self {
        Self {
            controller,
            anchor: SearchAnchor::Toolbar,
            rows: Vec::new(),
            selected_index: None,
        }
    }

    fn set_rows(
        &mut self,
        anchor: SearchAnchor,
        rows: Vec<SuggestionRow>,
        selected_index: Option<usize>,
        cx: &mut Context<Self>,
    ) {
        self.anchor = anchor;
        self.rows = rows;
        self.selected_index = selected_index;
        cx.notify();
    }
}

impl Render for SearchResultsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_dark = matches!(
            window.appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        let (fg, muted, border, hover_bg, selected_bg) = if is_dark {
            (
                rgb(0xf6f8fb),
                rgb(0xa2adbc),
                rgba(0xffffff14),
                rgba(0xffffff0a),
                rgba(0x4d90fe38),
            )
        } else {
            (
                rgb(0x162033),
                rgb(0x697489),
                rgba(0x0f172a14),
                rgba(0x0f172a08),
                rgba(0x0a84ff29),
            )
        };

        div()
            .id("search-results-scroll")
            .flex()
            .flex_col()
            .size_full()
            .overflow_y_scroll()
            .p_2()
            .gap_1()
            .child(
                div()
                    .px_2()
                    .pt_1()
                    .pb_2()
                    .border_b_1()
                    .border_color(border)
                    .child(div().text_xs().text_color(muted).child(self.anchor.title())),
            )
            .children(self.rows.iter().enumerate().map(|(index, row)| {
                let controller = self.controller.clone();
                let title = row.title.clone();
                let detail = row.detail.clone();
                let badge = row.kind.badge();
                let is_selected = self.selected_index == Some(index);

                div()
                    .id(("search-result-row", index))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .px_3()
                    .py_2()
                    .border_1()
                    .border_color(if is_selected {
                        selected_bg
                    } else {
                        rgba(0x00000000)
                    })
                    .rounded_lg()
                    .cursor_pointer()
                    .when(is_selected, |row| row.bg(selected_bg))
                    .when(!is_selected, |row| row.hover(|style| style.bg(hover_bg)))
                    .on_click(cx.listener(move |_, _, window, cx| {
                        window.dismiss_native_search_suggestion_menu();
                        let _ = controller.update(cx, |controller, cx| {
                            controller.activate_suggestion(index, window, cx);
                        });
                    }))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .child(div().text_sm().text_color(fg).child(title))
                            .child(div().text_xs().text_color(muted).child(badge)),
                    )
                    .child(div().text_xs().text_color(muted).child(detail))
            }))
    }
}

struct SearchSuggestionsExample {
    toolbar_installed: bool,
    toolbar_search_text: String,
    content_search_text: String,
    current_title: String,
    current_url: String,
    active_anchor: Option<SearchAnchor>,
    rows: Vec<SuggestionRow>,
    selected_index: Option<usize>,
    results_view: Option<Entity<SearchResultsView>>,
}

impl SearchSuggestionsExample {
    fn new() -> Self {
        Self {
            toolbar_installed: false,
            toolbar_search_text: String::new(),
            content_search_text: String::new(),
            current_title: "New Tab".to_string(),
            current_url: String::new(),
            active_anchor: None,
            rows: Vec::new(),
            selected_index: None,
            results_view: None,
        }
    }

    fn ensure_results_view(&mut self, cx: &mut Context<Self>) -> Entity<SearchResultsView> {
        if let Some(view) = &self.results_view {
            return view.clone();
        }

        let controller = cx.entity().downgrade();
        let view = cx.new(|_| SearchResultsView::new(controller));
        self.results_view = Some(view.clone());
        view
    }

    fn build_rows(query: &str) -> Vec<SuggestionRow> {
        let mut rows = Vec::new();
        let trimmed_query = query.trim();
        if !trimmed_query.is_empty() {
            let encoded: String =
                url::form_urlencoded::byte_serialize(trimmed_query.as_bytes()).collect();
            rows.push(SuggestionRow {
                kind: SuggestionKind::Search,
                title: format!("Search for “{}”", trimmed_query),
                detail: "Google".to_string(),
                url: format!("https://www.google.com/search?q={encoded}"),
            });
        }

        let query_lower = trimmed_query.to_lowercase();
        let mut matches: Vec<SuggestionRow> = SUGGESTIONS
            .iter()
            .filter(|entry| {
                trimmed_query.is_empty()
                    || entry.title.to_lowercase().contains(&query_lower)
                    || entry.detail.to_lowercase().contains(&query_lower)
                    || entry.url.to_lowercase().contains(&query_lower)
            })
            .map(|entry| SuggestionRow {
                kind: entry.kind,
                title: entry.title.to_string(),
                detail: entry.detail.to_string(),
                url: entry.url.to_string(),
            })
            .collect();

        matches.sort_by_key(|entry| match entry.kind {
            SuggestionKind::TopHit => 0,
            SuggestionKind::Bookmark => 1,
            SuggestionKind::History => 2,
            SuggestionKind::Search => 3,
        });
        matches.truncate(7);
        rows.extend(matches);
        rows
    }

    fn menu_height(row_count: usize) -> f64 {
        (HEADER_HEIGHT + row_count as f64 * ROW_HEIGHT + MENU_PADDING)
            .clamp(MENU_MIN_HEIGHT, MENU_MAX_HEIGHT)
    }

    fn query_for(&self, anchor: SearchAnchor) -> &str {
        match anchor {
            SearchAnchor::Toolbar => &self.toolbar_search_text,
            SearchAnchor::Content => &self.content_search_text,
        }
    }

    fn refresh_results_for(
        &mut self,
        anchor: SearchAnchor,
        query: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        eprintln!(
            "[native_search_suggestions] refresh_results_for anchor={anchor:?} query={query:?}"
        );
        match anchor {
            SearchAnchor::Toolbar => self.toolbar_search_text = query,
            SearchAnchor::Content => self.content_search_text = query,
        }

        self.active_anchor = Some(anchor);
        self.rows = Self::build_rows(self.query_for(anchor));
        self.selected_index = (!self.rows.is_empty()).then_some(0);

        if self.rows.is_empty() {
            self.dismiss_results(window, cx);
            return;
        }

        let view = self.ensure_results_view(cx);
        view.update(cx, |view, cx| {
            view.set_rows(anchor, self.rows.clone(), self.selected_index, cx);
        });

        let menu_height = Self::menu_height(self.rows.len());
        window.update_native_search_suggestion_menu(
            NativeSearchSuggestionMenu::new(MENU_WIDTH, menu_height).content_view(view),
            anchor.target(),
        );
        cx.notify();
    }

    fn dismiss_results(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        eprintln!(
            "[native_search_suggestions] dismiss_results active_anchor={:?} rows={}",
            self.active_anchor,
            self.rows.len()
        );
        window.dismiss_native_search_suggestion_menu();
        self.active_anchor = None;
        self.rows.clear();
        self.selected_index = None;
        cx.notify();
    }

    fn move_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        if self.rows.is_empty() {
            return;
        }

        let current_index = self.selected_index.unwrap_or(0) as isize;
        let len = self.rows.len() as isize;
        let next_index = (current_index + delta).rem_euclid(len) as usize;
        eprintln!(
            "[native_search_suggestions] move_selection delta={delta} current={current_index} next={next_index}"
        );
        self.selected_index = Some(next_index);
        if let Some(view) = &self.results_view {
            view.update(cx, |view, cx| {
                view.set_rows(
                    self.active_anchor.unwrap_or(SearchAnchor::Toolbar),
                    self.rows.clone(),
                    self.selected_index,
                    cx,
                );
            });
        }
        cx.notify();
    }

    fn submit_search(
        &mut self,
        anchor: SearchAnchor,
        text: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        eprintln!(
            "[native_search_suggestions] submit_search anchor={anchor:?} text={text:?} selected_index={:?}",
            self.selected_index
        );
        let submitted_text = if text.trim().is_empty() {
            self.query_for(anchor).trim().to_string()
        } else {
            text.trim().to_string()
        };

        if let Some(selected_index) = self.selected_index {
            self.activate_suggestion(selected_index, window, cx);
            return;
        }

        if submitted_text.is_empty() {
            self.dismiss_results(window, cx);
            return;
        }

        let encoded: String =
            url::form_urlencoded::byte_serialize(submitted_text.as_bytes()).collect();
        self.commit_navigation(
            format!("Search for “{}”", submitted_text),
            format!("https://www.google.com/search?q={encoded}"),
            window,
            cx,
        );
    }

    fn activate_suggestion(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        eprintln!("[native_search_suggestions] activate_suggestion index={index}");
        let Some(row) = self.rows.get(index).cloned() else {
            return;
        };
        self.commit_navigation(row.title, row.url, window, cx);
    }

    fn commit_navigation(
        &mut self,
        title: String,
        url: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        eprintln!("[native_search_suggestions] commit_navigation title={title:?} url={url:?}");
        self.current_title = title;
        self.current_url = url;
        self.content_search_text.clear();
        self.dismiss_results(window, cx);
    }
}

impl Render for SearchSuggestionsExample {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.toolbar_installed {
            let weak_back = cx.entity().downgrade();
            let weak_change = cx.entity().downgrade();
            let weak_submit = cx.entity().downgrade();
            let weak_move_up = cx.entity().downgrade();
            let weak_move_down = cx.entity().downgrade();
            let weak_cancel = cx.entity().downgrade();
            let weak_begin_editing = cx.entity().downgrade();
            let weak_end_editing = cx.entity().downgrade();

            window.set_native_toolbar(Some(
                NativeToolbar::new("search_suggestions.toolbar")
                    .title("Search Suggestions Demo")
                    .display_mode(NativeToolbarDisplayMode::IconOnly)
                    .shows_baseline_separator(true)
                    .item(NativeToolbarItem::Button(
                        NativeToolbarButton::new("focus-content", "Focus New Tab Search")
                            .icon("sidebar.left")
                            .tool_tip("Focus the new tab search field")
                            .on_click(move |_: &NativeToolbarClickEvent, window, cx| {
                                let _ = weak_back.update(cx, |_, _| {
                                    window.focus_native_search_field(
                                        NativeSearchFieldTarget::ContentElement(
                                            CONTENT_SEARCH_ID.into(),
                                        ),
                                        true,
                                    );
                                });
                            }),
                    ))
                    .item(NativeToolbarItem::SearchField(
                        NativeToolbarSearchField::new(TOOLBAR_SEARCH_ID)
                            .placeholder("Search or enter URL")
                            .min_width(px(280.0))
                            .max_width(px(520.0))
                            .preferred_width_for_search_field(px(520.0))
                            .resigns_first_responder_with_cancel(true)
                            .on_begin_editing(
                                move |event: &NativeToolbarSearchEvent, window, cx| {
                                    let text = event.text.clone();
                                    eprintln!(
                                        "[native_search_suggestions] toolbar on_begin_editing text={text:?}"
                                    );
                                    let _ = weak_begin_editing.update(cx, |this, cx| {
                                        this.refresh_results_for(
                                            SearchAnchor::Toolbar,
                                            text,
                                            window,
                                            cx,
                                        );
                                    });
                                },
                            )
                            .on_change(move |event: &NativeToolbarSearchEvent, window, cx| {
                                let text = event.text.clone();
                                eprintln!(
                                    "[native_search_suggestions] toolbar on_change text={text:?}"
                                );
                                let _ = weak_change.update(cx, |this, cx| {
                                    this.refresh_results_for(
                                        SearchAnchor::Toolbar,
                                        text,
                                        window,
                                        cx,
                                    );
                                });
                            })
                            .on_submit(move |event: &NativeToolbarSearchEvent, window, cx| {
                                let text = event.text.clone();
                                eprintln!(
                                    "[native_search_suggestions] toolbar on_submit text={text:?}"
                                );
                                let _ = weak_submit.update(cx, |this, cx| {
                                    this.submit_search(SearchAnchor::Toolbar, text, window, cx);
                                });
                            })
                            .on_move_up(move |_, _, cx| {
                                eprintln!("[native_search_suggestions] toolbar on_move_up");
                                let _ = weak_move_up.update(cx, |this, cx| {
                                    this.move_selection(-1, cx);
                                });
                            })
                            .on_move_down(move |_, _, cx| {
                                eprintln!("[native_search_suggestions] toolbar on_move_down");
                                let _ = weak_move_down.update(cx, |this, cx| {
                                    this.move_selection(1, cx);
                                });
                            })
                            .on_cancel(move |_, window, cx| {
                                eprintln!("[native_search_suggestions] toolbar on_cancel");
                                let _ = weak_cancel.update(cx, |this, cx| {
                                    this.dismiss_results(window, cx);
                                });
                            })
                            .on_end_editing(move |_, window, cx| {
                                eprintln!("[native_search_suggestions] toolbar on_end_editing");
                                let _ = weak_end_editing.update(cx, |this, cx| {
                                    if this.active_anchor == Some(SearchAnchor::Toolbar) {
                                        this.dismiss_results(window, cx);
                                    }
                                });
                            }),
                    )),
            ));
            self.toolbar_installed = true;
        }

        let is_dark = matches!(
            window.appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        let (bg, card_bg, fg, muted, card_border, accent) = if is_dark {
            (
                rgb(0x12161d),
                rgba(0xffffff0a),
                rgb(0xf6f8fb),
                rgb(0x9aa6b8),
                rgba(0xffffff14),
                rgb(0x68a4ff),
            )
        } else {
            (
                rgb(0xf4f6f9),
                rgba(0xffffffb2),
                rgb(0x192334),
                rgb(0x677489),
                rgba(0x0f172a14),
                rgb(0x0a84ff),
            )
        };
        let weak_content_focus = cx.entity().downgrade();
        let weak_content_change = cx.entity().downgrade();
        let weak_content_submit = cx.entity().downgrade();
        let weak_content_move_up = cx.entity().downgrade();
        let weak_content_move_down = cx.entity().downgrade();
        let weak_content_cancel = cx.entity().downgrade();
        let weak_content_blur = cx.entity().downgrade();

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(bg)
            .p_6()
            .gap_6()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().text_3xl().text_color(fg).child("Native Search Surfaces"))
                    .child(
                        div()
                            .text_base()
                            .text_color(muted)
                            .child(
                                "Toolbar search uses NSSearchToolbarItem. New-tab search uses NSSearchField. Both result surfaces use the native search suggestion menu anchored to the field.",
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p_5()
                    .bg(card_bg)
                    .border_1()
                    .border_color(card_border)
                    .rounded_xl()
                    .child(
                        div()
                            .text_sm()
                            .text_color(muted)
                            .child("New Tab Search"),
                    )
                    .child(
                        native_search_field(CONTENT_SEARCH_ID)
                            .placeholder("Search the web or jump to a page")
                            .value(self.content_search_text.clone())
                            .w(px(520.0))
                            .h(px(34.0))
                            .on_focus(move |window, cx| {
                                eprintln!("[native_search_suggestions] content on_focus");
                                let _ = weak_content_focus.update(cx, |this, cx| {
                                    this.refresh_results_for(
                                        SearchAnchor::Content,
                                        this.content_search_text.clone(),
                                        window,
                                        cx,
                                    );
                                });
                            })
                            .on_change(move |event: &SearchChangeEvent, window, cx| {
                                let text = event.text.clone();
                                eprintln!(
                                    "[native_search_suggestions] content on_change text={text:?}"
                                );
                                let _ = weak_content_change.update(cx, |this, cx| {
                                    this.refresh_results_for(
                                        SearchAnchor::Content,
                                        text,
                                        window,
                                        cx,
                                    );
                                });
                            })
                            .on_submit(move |event: &SearchSubmitEvent, window, cx| {
                                let text = event.text.clone();
                                eprintln!(
                                    "[native_search_suggestions] content on_submit text={text:?}"
                                );
                                let _ = weak_content_submit.update(cx, |this, cx| {
                                    this.submit_search(SearchAnchor::Content, text, window, cx);
                                });
                            })
                            .on_move_up(move |window, cx| {
                                eprintln!("[native_search_suggestions] content on_move_up");
                                let _ = weak_content_move_up.update(cx, |this, cx| {
                                    if this.active_anchor == Some(SearchAnchor::Content) {
                                        this.move_selection(-1, cx);
                                    } else {
                                        this.refresh_results_for(
                                            SearchAnchor::Content,
                                            this.content_search_text.clone(),
                                            window,
                                            cx,
                                        );
                                    }
                                });
                            })
                            .on_move_down(move |window, cx| {
                                eprintln!("[native_search_suggestions] content on_move_down");
                                let _ = weak_content_move_down.update(cx, |this, cx| {
                                    if this.active_anchor == Some(SearchAnchor::Content) {
                                        this.move_selection(1, cx);
                                    } else {
                                        this.refresh_results_for(
                                            SearchAnchor::Content,
                                            this.content_search_text.clone(),
                                            window,
                                            cx,
                                        );
                                    }
                                });
                            })
                            .on_cancel(move |window, cx| {
                                eprintln!("[native_search_suggestions] content on_cancel");
                                let _ = weak_content_cancel.update(cx, |this, cx| {
                                    this.dismiss_results(window, cx);
                                });
                            })
                            .on_blur(move |_: &SearchSubmitEvent, window, cx| {
                                eprintln!("[native_search_suggestions] content on_blur");
                                let _ = weak_content_blur.update(cx, |this, cx| {
                                    if this.active_anchor == Some(SearchAnchor::Content) {
                                        this.dismiss_results(window, cx);
                                    }
                                });
                            }),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(muted)
                            .child("Try “glass”, “apple”, or leave the field empty to see recent destinations."),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_5()
                    .bg(card_bg)
                    .border_1()
                    .border_color(card_border)
                    .rounded_xl()
                    .child(div().text_sm().text_color(muted).child("Current Page"))
                    .child(
                        div()
                            .text_xl()
                            .text_color(fg)
                            .child(self.current_title.clone()),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(accent)
                            .child(if self.current_url.is_empty() {
                                "Nothing selected yet".to_string()
                            } else {
                                self.current_url.clone()
                            }),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(muted)
                            .child(match self.active_anchor {
                                Some(anchor) => format!(
                                    "{} menu open with {} results",
                                    anchor.title(),
                                    self.rows.len()
                                ),
                                None => "Type in either field to open the anchored search suggestion menu."
                                    .to_string(),
                            }),
                    ),
            )
    }
}

fn main() {
    gpui_platform::application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1120.0), px(760.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| SearchSuggestionsExample::new()),
        )
        .unwrap();

        cx.activate(true);
    });
}
