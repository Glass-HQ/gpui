use gpui::{
    App, Bounds, ClipboardEntry, ClipboardItem, Context, ElementInputHandler, EntityInputHandler,
    ExternalPaths, FocusHandle, Focusable, Image, ImageFormat, MouseButton, PathPromptOptions,
    PinchEvent, Pixels, Point, RotationEvent, TextInputAutocapitalization,
    TextInputAutocorrection, TextInputConfig, TextInputContentType,
    TextInputKeyboardAppearance, TextInputKeyboardType, TextInputReturnKeyType,
    TextInputSoftKeyboardPolicy, TextInputSpellChecking, TextInputSubmitBehavior, UTF16Selection,
    Window, WindowAppearance, WindowOptions, canvas, div, prelude::*, px, rgb, rgba,
};
use std::ops::Range;

pub type AppLauncher = dyn Fn(Box<dyn FnOnce(&mut App)>);
const IOS_CLIPBOARD_TEST_PNG: [u8; 68] = [
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x04, 0x00, 0x00, 0x00, 0xb5, 0x1c, 0x0c,
    0x02, 0x00, 0x00, 0x00, 0x0b, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0xfc, 0xff, 0x1f, 0x00,
    0x03, 0x03, 0x02, 0x00, 0xef, 0x97, 0xd9, 0x77, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44,
    0xae, 0x42, 0x60, 0x82,
];

#[derive(Clone, Copy, Debug)]
pub struct IosDemoDescriptor {
    pub name: &'static str,
    pub description: &'static str,
}

pub const IOS_DEMOS: &[IosDemoDescriptor] = &[
    IosDemoDescriptor {
        name: "hello_world",
        description: "Colored boxes",
    },
    IosDemoDescriptor {
        name: "touch",
        description: "Tappable boxes with tap counter",
    },
    IosDemoDescriptor {
        name: "text",
        description: "Text rendering at various sizes",
    },
    IosDemoDescriptor {
        name: "lifecycle",
        description: "Window size, appearance, resize counting",
    },
    IosDemoDescriptor {
        name: "combined",
        description: "Touch + text + lifecycle + dark/light mode",
    },
    IosDemoDescriptor {
        name: "scroll",
        description: "Two-finger pan scrollable list (50 items)",
    },
    IosDemoDescriptor {
        name: "text_input",
        description: "UIKit-backed text input validation lab",
    },
    IosDemoDescriptor {
        name: "vertical_scroll",
        description: "Single-finger vertical scroll (100 items)",
    },
    IosDemoDescriptor {
        name: "horizontal_scroll",
        description: "Single-finger horizontal card strip",
    },
    IosDemoDescriptor {
        name: "pinch",
        description: "Pinch gesture to scale (0.25x–5x)",
    },
    IosDemoDescriptor {
        name: "rotation",
        description: "Two-finger rotation with color shift",
    },
    IosDemoDescriptor {
        name: "controls",
        description: "GPUI-painted controls demo",
    },
    IosDemoDescriptor {
        name: "safe_area",
        description: "Visual safe area inset debugger",
    },
    IosDemoDescriptor {
        name: "layout_showcase",
        description: "Layout API showcase",
    },
    IosDemoDescriptor {
        name: "file_picker",
        description: "Open/save picker validation",
    },
    IosDemoDescriptor {
        name: "clipboard",
        description: "Rich clipboard validation",
    },
    IosDemoDescriptor {
        name: "file_drop",
        description: "External file drop validation",
    },
];

#[derive(Debug)]
pub struct UnknownIosDemo<'a> {
    pub name: &'a str,
}

fn run_ios_app<V: Render + 'static>(
    launch: &AppLauncher,
    build_view: impl FnOnce(&mut Window, &mut Context<V>) -> V + 'static,
) {
    launch(Box::new(move |cx: &mut App| {
        cx.open_window(WindowOptions::default(), |window, cx| {
            cx.new(|cx| build_view(window, cx))
        })
        .expect("failed to open GPUI iOS window");
        cx.activate(true);
    }));
}

fn action_button(
    id: impl Into<String>,
    label: &str,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id.into())
        .flex_none()
        .px(px(12.0))
        .py(px(8.0))
        .rounded(px(8.0))
        .bg(rgb(0x313244))
        .border_1()
        .border_color(rgb(0x45475a))
        .text_color(rgb(0xcdd6f4))
        .cursor_pointer()
        .active(|this| this.opacity(0.85))
        .on_click(on_click)
        .child(label.to_string())
}

// ---------------------------------------------------------------------------
// 1. Hello World — original colored boxes demo
// ---------------------------------------------------------------------------

struct IosHelloWorld;

impl Render for IosHelloWorld {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let safe = window.safe_area_insets();
        div()
            .size_full()
            .pt(safe.top)
            .pb(safe.bottom)
            .pl(safe.left)
            .pr(safe.right)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(20.0))
            .bg(rgb(0x1e1e2e))
            .child(
                div()
                    .w(px(200.0))
                    .h(px(80.0))
                    .bg(rgb(0xf38ba8))
                    .rounded(px(12.0)),
            )
            .child(
                div()
                    .w(px(200.0))
                    .h(px(80.0))
                    .bg(rgb(0xa6e3a1))
                    .rounded(px(12.0)),
            )
            .child(
                div()
                    .w(px(200.0))
                    .h(px(80.0))
                    .bg(rgb(0x89b4fa))
                    .rounded(px(12.0)),
            )
    }
}

pub fn run_hello_world(launch: &AppLauncher) {
    run_ios_app(launch, |_, _| IosHelloWorld);
}

// ---------------------------------------------------------------------------
// 2. Touch Input Demo — tappable boxes that change color on tap
// ---------------------------------------------------------------------------

struct IosTouchDemo {
    tapped_box: Option<usize>,
    tap_count: usize,
}

impl Render for IosTouchDemo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let safe = window.safe_area_insets();
        let tapped = self.tapped_box;
        let tap_count = self.tap_count;

        let box_color = |index: usize, base: u32, active: u32| -> u32 {
            if tapped == Some(index) { active } else { base }
        };

        div()
            .size_full()
            .pt(safe.top)
            .pb(safe.bottom)
            .pl(safe.left)
            .pr(safe.right)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(20.0))
            .bg(rgb(0x1e1e2e))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(280.0))
                    .h(px(40.0))
                    .bg(rgb(0x313244))
                    .rounded(px(8.0))
                    .child(format!("Tap a box! (taps: {})", tap_count)),
            )
            .child(
                div()
                    .id("box-0")
                    .w(px(200.0))
                    .h(px(80.0))
                    .bg(rgb(box_color(0, 0xf38ba8, 0xff5577)))
                    .rounded(px(12.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child("Red")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            log::info!("touch: red box tapped");
                            this.tapped_box = Some(0);
                            this.tap_count += 1;
                            cx.notify();
                        }),
                    ),
            )
            .child(
                div()
                    .id("box-1")
                    .w(px(200.0))
                    .h(px(80.0))
                    .bg(rgb(box_color(1, 0xa6e3a1, 0x55ff77)))
                    .rounded(px(12.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child("Green")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            log::info!("touch: green box tapped");
                            this.tapped_box = Some(1);
                            this.tap_count += 1;
                            cx.notify();
                        }),
                    ),
            )
            .child(
                div()
                    .id("box-2")
                    .w(px(200.0))
                    .h(px(80.0))
                    .bg(rgb(box_color(2, 0x89b4fa, 0x5577ff)))
                    .rounded(px(12.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child("Blue")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            log::info!("touch: blue box tapped");
                            this.tapped_box = Some(2);
                            this.tap_count += 1;
                            cx.notify();
                        }),
                    ),
            )
    }
}

pub fn run_touch_demo(launch: &AppLauncher) {
    run_ios_app(launch, |_, _| IosTouchDemo {
        tapped_box: None,
        tap_count: 0,
    });
}

// ---------------------------------------------------------------------------
// 3. Text Rendering Demo — text at various sizes
// ---------------------------------------------------------------------------

struct IosTextDemo;

impl Render for IosTextDemo {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let safe = window.safe_area_insets();
        div()
            .size_full()
            .pt(safe.top)
            .pb(safe.bottom)
            .pl(safe.left)
            .pr(safe.right)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(16.0))
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .child(div().text_size(px(32.0)).child("Hello iOS!"))
            .child(div().text_size(px(20.0)).child("CoreText text rendering"))
            .child(
                div()
                    .text_size(px(16.0))
                    .text_color(rgb(0xa6adc8))
                    .child("The quick brown fox jumps over the lazy dog"),
            )
            .child(
                div()
                    .text_size(px(14.0))
                    .text_color(rgb(0x6c7086))
                    .child("ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
            )
            .child(
                div()
                    .text_size(px(14.0))
                    .text_color(rgb(0x6c7086))
                    .child("abcdefghijklmnopqrstuvwxyz"),
            )
            .child(
                div()
                    .text_size(px(14.0))
                    .text_color(rgb(0x6c7086))
                    .child("0123456789 !@#$%^&*()"),
            )
    }
}

pub fn run_text_demo(launch: &AppLauncher) {
    run_ios_app(launch, |_, _| IosTextDemo);
}

// ---------------------------------------------------------------------------
// 4. Window Lifecycle Demo — shows active state, appearance, and size
// ---------------------------------------------------------------------------

struct IosLifecycleDemo {
    resize_count: usize,
}

impl Render for IosLifecycleDemo {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let safe = window.safe_area_insets();
        let bounds = window.bounds();
        let appearance = window.appearance();
        let scale = window.scale_factor();

        let appearance_name = format!("{:?}", appearance);
        let size_text = format!(
            "{:.0}x{:.0} @{:.0}x",
            f32::from(bounds.size.width),
            f32::from(bounds.size.height),
            scale,
        );

        div()
            .size_full()
            .pt(safe.top)
            .pb(safe.bottom)
            .pl(safe.left)
            .pr(safe.right)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(16.0))
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .child(div().text_size(px(24.0)).child("Window Lifecycle"))
            .child(
                div()
                    .w(px(300.0))
                    .p(px(16.0))
                    .bg(rgb(0x313244))
                    .rounded(px(12.0))
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(16.0))
                            .child(format!("Appearance: {}", appearance_name)),
                    )
                    .child(
                        div()
                            .text_size(px(16.0))
                            .child(format!("Size: {}", size_text)),
                    )
                    .child(
                        div()
                            .text_size(px(16.0))
                            .child(format!("Resizes: {}", self.resize_count)),
                    )
                    .child(
                        div()
                            .text_size(px(14.0))
                            .text_color(rgb(0x6c7086))
                            .child("Rotate device or toggle dark mode to see changes"),
                    ),
            )
    }
}

pub fn run_lifecycle_demo(launch: &AppLauncher) {
    run_ios_app(launch, |_, _| IosLifecycleDemo { resize_count: 0 });
}

// ---------------------------------------------------------------------------
// 5. Combined Demo — touch + text + lifecycle info in one view
// ---------------------------------------------------------------------------

struct IosCombinedDemo {
    tap_count: usize,
    last_tapped: Option<&'static str>,
}

impl Render for IosCombinedDemo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let safe = window.safe_area_insets();
        let bounds = window.bounds();
        let appearance = window.appearance();
        let scale = window.scale_factor();
        let tap_count = self.tap_count;
        let last_tapped = self.last_tapped.unwrap_or("none");

        let is_dark = matches!(
            appearance,
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        let bg_color = if is_dark {
            rgb(0x1e1e2e)
        } else {
            rgb(0xeff1f5)
        };
        let text_color = if is_dark {
            rgb(0xcdd6f4)
        } else {
            rgb(0x4c4f69)
        };
        let panel_bg = if is_dark {
            rgb(0x313244)
        } else {
            rgb(0xccd0da)
        };
        let muted_text = if is_dark {
            rgb(0x6c7086)
        } else {
            rgb(0x9ca0b0)
        };

        div()
            .size_full()
            .pt(safe.top)
            .pb(safe.bottom)
            .pl(safe.left)
            .pr(safe.right)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(12.0))
            .bg(bg_color)
            .text_color(text_color)
            // Title
            .child(div().text_size(px(28.0)).child("GPUI on iOS"))
            // Info panel
            .child(
                div()
                    .w(px(300.0))
                    .p(px(12.0))
                    .bg(panel_bg)
                    .rounded(px(8.0))
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(div().text_size(px(14.0)).child(format!(
                        "{:.0}x{:.0} @{:.0}x  {:?}",
                        f32::from(bounds.size.width),
                        f32::from(bounds.size.height),
                        scale,
                        appearance,
                    )))
                    .child(
                        div()
                            .text_size(px(14.0))
                            .child(format!("Taps: {}  Last: {}", tap_count, last_tapped)),
                    ),
            )
            // Tappable boxes
            .child(
                div()
                    .id("red")
                    .w(px(200.0))
                    .h(px(60.0))
                    .bg(rgb(0xf38ba8))
                    .rounded(px(12.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child("Tap me")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.tap_count += 1;
                            this.last_tapped = Some("red");
                            cx.notify();
                        }),
                    ),
            )
            .child(
                div()
                    .id("green")
                    .w(px(200.0))
                    .h(px(60.0))
                    .bg(rgb(0xa6e3a1))
                    .rounded(px(12.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(0x1e1e2e))
                    .child("Tap me")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.tap_count += 1;
                            this.last_tapped = Some("green");
                            cx.notify();
                        }),
                    ),
            )
            .child(
                div()
                    .id("blue")
                    .w(px(200.0))
                    .h(px(60.0))
                    .bg(rgb(0x89b4fa))
                    .rounded(px(12.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(0x1e1e2e))
                    .child("Tap me")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.tap_count += 1;
                            this.last_tapped = Some("blue");
                            cx.notify();
                        }),
                    ),
            )
            // Text samples
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(muted_text)
                    .child("The quick brown fox jumps over the lazy dog"),
            )
    }
}

pub fn run_combined_demo(launch: &AppLauncher) {
    run_ios_app(launch, |_, _| IosCombinedDemo {
        tap_count: 0,
        last_tapped: None,
    });
}

// ---------------------------------------------------------------------------
// 6. Scroll Demo — two-finger pan scrollable list
// ---------------------------------------------------------------------------

struct IosScrollDemo;

impl Render for IosScrollDemo {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let safe = window.safe_area_insets();
        let colors = [
            0xf38ba8u32,
            0xa6e3a1,
            0x89b4fa,
            0xfab387,
            0xcba6f7,
            0xf9e2af,
            0x94e2d5,
            0xf2cdcd,
            0x89dceb,
            0xb4befe,
        ];

        let mut scroll_content = div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(16.0))
            .pb(safe.bottom);

        for i in 0..50 {
            let color = colors[i % colors.len()];
            scroll_content = scroll_content.child(
                div()
                    .w_full()
                    .h(px(60.0))
                    .bg(rgb(color))
                    .rounded(px(8.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(0x1e1e2e))
                    .child(format!("Item {}", i + 1)),
            );
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .pl(safe.left)
            .pr(safe.right)
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .child(
                div()
                    .w_full()
                    .pt(safe.top)
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_end()
                    .bg(rgb(0x313244))
                    .child(
                        div()
                            .h(px(60.0))
                            .flex()
                            .items_center()
                            .text_size(px(20.0))
                            .child("Scroll Demo (2-finger pan)"),
                    ),
            )
            .child(
                div()
                    .id("scroll-container")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(scroll_content),
            )
    }
}

pub fn run_scroll_demo(launch: &AppLauncher) {
    run_ios_app(launch, |_, _| IosScrollDemo);
}

// ---------------------------------------------------------------------------
// 7. Vertical Scroll Demo — single-finger scrollable list with momentum
// ---------------------------------------------------------------------------

struct IosVerticalScrollDemo;

impl Render for IosVerticalScrollDemo {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let safe = window.safe_area_insets();
        let colors = [
            0xf38ba8u32,
            0xa6e3a1,
            0x89b4fa,
            0xfab387,
            0xcba6f7,
            0xf9e2af,
            0x94e2d5,
            0xf2cdcd,
            0x89dceb,
            0xb4befe,
        ];

        let mut list = div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(16.0))
            .pb(safe.bottom);

        for i in 0..100 {
            let color = colors[i % colors.len()];
            list = list.child(
                div()
                    .w_full()
                    .h(px(56.0))
                    .bg(rgb(color))
                    .rounded(px(8.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(0x1e1e2e))
                    .child(format!("Row {}", i + 1)),
            );
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .pl(safe.left)
            .pr(safe.right)
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .child(
                div()
                    .w_full()
                    .pt(safe.top)
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_end()
                    .bg(rgb(0x313244))
                    .child(
                        div()
                            .h(px(60.0))
                            .flex()
                            .items_center()
                            .text_size(px(20.0))
                            .child("Vertical Scroll (1-finger)"),
                    ),
            )
            .child(div().id("vscroll").flex_1().overflow_y_scroll().child(list))
    }
}

pub fn run_vertical_scroll_demo(launch: &AppLauncher) {
    run_ios_app(launch, |_, _| IosVerticalScrollDemo);
}

// ---------------------------------------------------------------------------
// 9. Horizontal Scroll Demo — single-finger horizontal scroll
// ---------------------------------------------------------------------------

struct IosHorizontalScrollDemo;

impl Render for IosHorizontalScrollDemo {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let safe = window.safe_area_insets();
        let colors = [
            0xf38ba8u32,
            0xa6e3a1,
            0x89b4fa,
            0xfab387,
            0xcba6f7,
            0xf9e2af,
            0x94e2d5,
            0xf2cdcd,
            0x89dceb,
            0xb4befe,
        ];

        let card_count = 30;
        let card_w = 140.0;
        let gap = 12.0;
        let pad = 16.0;
        let total_w = (card_count as f32) * card_w + ((card_count - 1) as f32) * gap + pad * 2.0;

        let mut strip = div()
            .flex()
            .flex_row()
            .gap(px(gap))
            .p(px(pad))
            .min_w(px(total_w));

        for i in 0..card_count {
            let color = colors[i % colors.len()];
            strip = strip.child(
                div()
                    .w(px(140.0))
                    .h(px(180.0))
                    .flex_shrink_0()
                    .bg(rgb(color))
                    .rounded(px(12.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(8.0))
                    .text_color(rgb(0x1e1e2e))
                    .child(div().text_size(px(24.0)).child(format!("{}", i + 1)))
                    .child(div().text_size(px(14.0)).child(format!("Card {}", i + 1))),
            );
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .child(
                div()
                    .w_full()
                    .pt(safe.top)
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_end()
                    .bg(rgb(0x313244))
                    .child(
                        div()
                            .h(px(60.0))
                            .flex()
                            .items_center()
                            .text_size(px(20.0))
                            .child("Horizontal Scroll (1-finger)"),
                    ),
            )
            .child(
                div().flex_1().pb(safe.bottom).flex().items_center().child(
                    div()
                        .id("hscroll")
                        .w_full()
                        .h(px(220.0))
                        .overflow_x_scroll()
                        .child(strip),
                ),
            )
    }
}

pub fn run_horizontal_scroll_demo(launch: &AppLauncher) {
    run_ios_app(launch, |_, _| IosHorizontalScrollDemo);
}

// ---------------------------------------------------------------------------
// 10. Pinch Gesture Demo — pinch to scale a colored square
// ---------------------------------------------------------------------------

struct IosPinchDemo {
    scale: f32,
}

impl Render for IosPinchDemo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let safe = window.safe_area_insets();
        let scale = self.scale;
        let size = 120.0 * scale;

        div()
            .id("pinch-root")
            .size_full()
            .pt(safe.top)
            .pb(safe.bottom)
            .pl(safe.left)
            .pr(safe.right)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(24.0))
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .on_pinch(cx.listener(|this: &mut Self, event: &PinchEvent, _, cx| {
                this.scale *= 1.0 + event.delta;
                this.scale = this.scale.clamp(0.25, 5.0);
                cx.notify();
            }))
            .child(div().text_size(px(24.0)).child("Pinch to Scale"))
            .child(
                div()
                    .w(px(size))
                    .h(px(size))
                    .bg(rgb(0xcba6f7))
                    .rounded(px(12.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(0x1e1e2e))
                    .child(div().text_size(px(16.0)).child(format!("{:.1}x", scale))),
            )
            .child(
                div()
                    .text_size(px(14.0))
                    .text_color(rgb(0x6c7086))
                    .child("Use two fingers to pinch in/out"),
            )
    }
}

pub fn run_pinch_demo(launch: &AppLauncher) {
    run_ios_app(launch, |_, _| IosPinchDemo { scale: 1.0 });
}

// ---------------------------------------------------------------------------
// 11. Rotation Gesture Demo — two-finger rotate a colored rectangle
// ---------------------------------------------------------------------------

struct IosRotationDemo {
    angle_rad: f32,
}

impl Render for IosRotationDemo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let safe = window.safe_area_insets();
        let angle_deg = self.angle_rad.to_degrees();

        // Map angle to a hue shift for visual feedback
        let hue = ((angle_deg % 360.0 + 360.0) % 360.0) / 360.0;
        let (r, g, b) = hsv_to_rgb(hue, 0.6, 0.95);
        let box_color = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);

        div()
            .id("rotation-root")
            .size_full()
            .pt(safe.top)
            .pb(safe.bottom)
            .pl(safe.left)
            .pr(safe.right)
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(24.0))
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .on_rotation(
                cx.listener(|this: &mut Self, event: &RotationEvent, _, cx| {
                    this.angle_rad += event.rotation;
                    cx.notify();
                }),
            )
            .child(div().text_size(px(24.0)).child("Two-Finger Rotate"))
            .child(
                div()
                    .w(px(160.0))
                    .h(px(100.0))
                    .bg(rgb(box_color))
                    .rounded(px(12.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(0x1e1e2e))
                    .child(
                        div()
                            .text_size(px(20.0))
                            .child(format!("{:.1}\u{00b0}", angle_deg)),
                    ),
            )
            .child(
                div()
                    .text_size(px(14.0))
                    .text_color(rgb(0x6c7086))
                    .child("Color shifts as you rotate"),
            )
    }
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h * 6.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

fn previous_utf8_boundary(text: &str, offset: usize) -> usize {
    text[..offset]
        .char_indices()
        .last()
        .map(|(index, _)| index)
        .unwrap_or(0)
}

pub fn run_rotation_demo(launch: &AppLauncher) {
    run_ios_app(launch, |_, _| IosRotationDemo { angle_rad: 0.0 });
}

// ---------------------------------------------------------------------------
// 12. Controls Demo — GPU-painted GPUI controls on iOS
// ---------------------------------------------------------------------------

struct IosControlsDemo {
    focus_handle: FocusHandle,
    button_tap_count: usize,
    switch_on: bool,
    checkbox_checked: bool,
    slider_value: f32,
    stepper_value: i32,
    text_field_value: String,
    text_field_selection: Range<usize>,
    text_field_marked_range: Option<Range<usize>>,
    selected_segment: usize,
}

impl Focusable for IosControlsDemo {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl IosControlsDemo {
    fn utf16_offset_to_utf8(&self, utf16_offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;

        for ch in self.text_field_value.chars() {
            if utf16_count >= utf16_offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }

        utf8_offset
    }

    fn utf8_offset_to_utf16(&self, utf8_offset: usize) -> usize {
        let mut utf8_count = 0;
        let mut utf16_offset = 0;

        for ch in self.text_field_value.chars() {
            if utf8_count >= utf8_offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }

        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.utf8_offset_to_utf16(range.start)..self.utf8_offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.utf16_offset_to_utf8(range_utf16.start)..self.utf16_offset_to_utf8(range_utf16.end)
    }
}

impl EntityInputHandler for IosControlsDemo {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        adjusted_range.replace(self.range_to_utf16(&range));
        Some(self.text_field_value[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.text_field_selection),
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.text_field_marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.text_field_marked_range = None;
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let replacement_range = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or(self.text_field_marked_range.clone())
            .unwrap_or_else(|| self.text_field_selection.clone());

        self.text_field_value = format!(
            "{}{}{}",
            &self.text_field_value[..replacement_range.start],
            text,
            &self.text_field_value[replacement_range.end..]
        );
        let cursor = replacement_range.start + text.len();
        self.text_field_selection = cursor..cursor;
        self.text_field_marked_range = None;
        cx.notify();
    }

    fn delete_backward(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.text_field_marked_range.is_some() || !self.text_field_selection.is_empty() {
            self.replace_text_in_range(None, "", window, cx);
            return;
        }

        let cursor = self.text_field_selection.end;
        if cursor == 0 {
            return;
        }

        let start = previous_utf8_boundary(&self.text_field_value, cursor);
        self.replace_text_in_range(Some(start..cursor), "", window, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let replacement_range = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or(self.text_field_marked_range.clone())
            .unwrap_or_else(|| self.text_field_selection.clone());

        self.text_field_value = format!(
            "{}{}{}",
            &self.text_field_value[..replacement_range.start],
            new_text,
            &self.text_field_value[replacement_range.end..]
        );

        if new_text.is_empty() {
            self.text_field_marked_range = None;
        } else {
            self.text_field_marked_range =
                Some(replacement_range.start..replacement_range.start + new_text.len());
        }

        self.text_field_selection = new_selected_range
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .map(|range| replacement_range.start + range.start..replacement_range.start + range.end)
            .unwrap_or_else(|| {
                let cursor = replacement_range.start + new_text.len();
                cursor..cursor
            });

        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        Some(element_bounds)
    }

    fn character_index_for_point(
        &mut self,
        _point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        Some(self.text_field_value.len())
    }

    fn set_selected_text_range(
        &mut self,
        selection: Option<UTF16Selection>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(selection) = selection {
            self.text_field_selection = self.range_from_utf16(&selection.range);
            cx.notify();
        }
    }
}

impl Render for IosControlsDemo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let safe = window.safe_area_insets();
        let button_tap_count = self.button_tap_count;
        let switch_on = self.switch_on;
        let checkbox_checked = self.checkbox_checked;
        let slider_value = self.slider_value;
        let stepper_value = self.stepper_value;
        let text_field_value = self.text_field_value.clone();
        let selected_segment = self.selected_segment;

        let slider_percent = (slider_value * 100.0).round() as i32;
        let progress_value = slider_value;
        let focused = self.focus_handle.is_focused(window);

        fn row(label: &str, control: impl IntoElement) -> gpui::Div {
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .w_full()
                .gap(px(12.0))
                .child(
                    div()
                        .text_size(px(15.0))
                        .text_color(rgb(0xcdd6f4))
                        .flex_shrink_0()
                        .child(label.to_string()),
                )
                .child(control)
        }

        let section = |title: &str| {
            div().w_full().pt(px(16.0)).pb(px(4.0)).child(
                div()
                    .text_size(px(12.0))
                    .text_color(rgb(0x6c7086))
                    .child(title.to_string()),
            )
        };

        let mut slider_ticks = div().flex().flex_row().gap(px(2.0));
        for i in 0..=20 {
            let tick_value = i as f32 / 20.0;
            let active = tick_value <= slider_value + 0.0001;
            let tick_color = if active { 0x89b4fa } else { 0x45475a };
            slider_ticks = slider_ticks.child(
                div()
                    .w(px(8.0))
                    .h(px(18.0))
                    .rounded(px(2.0))
                    .bg(rgb(tick_color))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.slider_value = tick_value;
                            cx.notify();
                        }),
                    ),
            );
        }

        let text_border_color = if focused { 0x89b4fa } else { 0x585b70 };
        let text_display = if text_field_value.is_empty() && !focused {
            "Tap here to type...".to_string()
        } else {
            text_field_value.clone()
        };
        let text_input_entity = cx.entity();

        let mut toggle_group = div()
            .flex()
            .flex_row()
            .rounded(px(8.0))
            .border_1()
            .border_color(rgb(0x585b70))
            .overflow_hidden();

        for (index, label) in ["One", "Two", "Three"].iter().enumerate() {
            let is_selected = selected_segment == index;
            let bg = if is_selected { 0x89b4fa } else { 0x313244 };
            let fg = if is_selected { 0x1e1e2e } else { 0xcdd6f4 };
            toggle_group = toggle_group.child(
                div()
                    .px(px(12.0))
                    .py(px(6.0))
                    .bg(rgb(bg))
                    .text_color(rgb(fg))
                    .child((*label).to_string())
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.selected_segment = index;
                            cx.notify();
                        }),
                    ),
            );
        }

        let mut content = div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .p(px(20.0))
            .pb(safe.bottom)
            .w_full();

        content = content.child(
            div()
                .text_size(px(24.0))
                .text_color(rgb(0xcdd6f4))
                .pb(px(8.0))
                .child("Controls"),
        );

        content = content.child(section("BUTTON")).child(row(
            &format!("Taps: {}", button_tap_count),
            div()
                .px(px(12.0))
                .py(px(8.0))
                .rounded(px(8.0))
                .bg(rgb(0x89b4fa))
                .text_color(rgb(0x1e1e2e))
                .child("Tap Me")
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.button_tap_count += 1;
                        cx.notify();
                    }),
                ),
        ));

        content = content.child(section("SWITCH")).child(row(
            &format!("Switch: {}", if switch_on { "ON" } else { "OFF" }),
            div()
                .w(px(52.0))
                .h(px(30.0))
                .rounded(px(15.0))
                .bg(rgb(if switch_on { 0xa6e3a1 } else { 0x585b70 }))
                .flex()
                .items_center()
                .justify_start()
                .child(
                    div()
                        .w(px(22.0))
                        .h(px(22.0))
                        .ml(if switch_on { px(27.0) } else { px(3.0) })
                        .rounded(px(11.0))
                        .bg(rgb(0xf5e0dc)),
                )
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.switch_on = !this.switch_on;
                        cx.notify();
                    }),
                ),
        ));

        content = content.child(section("CHECKBOX")).child(row(
            &format!("Checked: {}", checkbox_checked),
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .w(px(20.0))
                        .h(px(20.0))
                        .rounded(px(4.0))
                        .border_1()
                        .border_color(rgb(0x89b4fa))
                        .bg(rgb(if checkbox_checked { 0x89b4fa } else { 0x1e1e2e }))
                        .text_color(rgb(0x1e1e2e))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(if checkbox_checked { "✓" } else { "" }),
                )
                .child("Enable feature")
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.checkbox_checked = !this.checkbox_checked;
                        cx.notify();
                    }),
                ),
        ));

        content = content.child(section("SLIDER")).child(row(
            &format!("Value: {}%", slider_percent),
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(slider_ticks)
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(0x6c7086))
                        .child("Tap ticks to adjust"),
                ),
        ));

        content = content.child(section("STEPPER")).child(row(
            &format!("Count: {}", stepper_value),
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .w(px(28.0))
                        .h(px(28.0))
                        .rounded(px(6.0))
                        .bg(rgb(0x313244))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child("-")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.stepper_value = (this.stepper_value - 1).clamp(0, 20);
                                cx.notify();
                            }),
                        ),
                )
                .child(
                    div()
                        .w(px(48.0))
                        .text_center()
                        .child(stepper_value.to_string()),
                )
                .child(
                    div()
                        .w(px(28.0))
                        .h(px(28.0))
                        .rounded(px(6.0))
                        .bg(rgb(0x313244))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child("+")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.stepper_value = (this.stepper_value + 1).clamp(0, 20);
                                cx.notify();
                            }),
                        ),
                ),
        ));

        content = content.child(section("TEXT FIELD")).child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .w_full()
                        .h(px(40.0))
                        .relative()
                        .px(px(10.0))
                        .bg(rgb(0x313244))
                        .rounded(px(8.0))
                        .border_1()
                        .border_color(rgb(text_border_color))
                        .flex()
                        .items_center()
                        .child(
                            div()
                                .text_size(px(14.0))
                                .text_color(if text_field_value.is_empty() && !focused {
                                    rgb(0x6c7086)
                                } else {
                                    rgb(0xcdd6f4)
                                })
                                .child(text_display),
                        )
                        .child(
                            canvas(
                                |_bounds, _window, _cx| (),
                                move |bounds, (), window, cx| {
                                    window.handle_input(
                                        &text_input_entity.read(cx).focus_handle.clone(),
                                        ElementInputHandler::new(bounds, text_input_entity.clone()),
                                        cx,
                                    );
                                },
                            )
                            .absolute()
                            .top_0()
                            .left_0()
                            .right_0()
                            .bottom_0(),
                        )
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|_this, _, window, cx| {
                                cx.focus_self(window);
                                cx.notify();
                            }),
                        ),
                )
                .child(div().text_size(px(12.0)).text_color(rgb(0x6c7086)).child(
                    if text_field_value.is_empty() {
                        "No text entered".to_string()
                    } else {
                        format!("Text: {}", text_field_value)
                    },
                )),
        );

        content = content.child(section("PROGRESS BAR")).child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(
                    div()
                        .w_full()
                        .h(px(12.0))
                        .rounded(px(6.0))
                        .bg(rgb(0x45475a))
                        .child(
                            div()
                                .w(px(progress_value * 260.0))
                                .h(px(12.0))
                                .rounded(px(6.0))
                                .bg(rgb(0xa6e3a1)),
                        ),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(0x6c7086))
                        .child(format!("{}% (driven by slider)", slider_percent)),
                ),
        );

        content = content.child(section("TOGGLE GROUP")).child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(toggle_group)
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(0x6c7086))
                        .child(format!(
                            "Selected: {} ({})",
                            ["One", "Two", "Three"][selected_segment],
                            selected_segment,
                        )),
                ),
        );

        content = content.child(section("IMAGE (GPU-PAINTED)")).child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(16.0))
                .child(
                    div()
                        .w(px(40.0))
                        .h(px(40.0))
                        .rounded(px(20.0))
                        .bg(rgb(0x89b4fa))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child("🌍"),
                )
                .child(
                    div()
                        .w(px(40.0))
                        .h(px(40.0))
                        .rounded(px(20.0))
                        .bg(rgb(0xf9e2af))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child("⭐"),
                )
                .child(
                    div()
                        .w(px(40.0))
                        .h(px(40.0))
                        .rounded(px(20.0))
                        .bg(rgb(0xf2cdcd))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child("❤"),
                )
                .child(
                    div()
                        .text_size(px(14.0))
                        .text_color(rgb(0xa6adc8))
                        .child("GPUI painted"),
                ),
        );

        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .pl(safe.left)
            .pr(safe.right)
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .child(
                div()
                    .w_full()
                    .pt(safe.top)
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_end()
                    .bg(rgb(0x313244))
                    .child(
                        div()
                            .h(px(60.0))
                            .flex()
                            .items_center()
                            .child(div().text_size(px(20.0)).child("Controls Demo")),
                    ),
            )
            .child(
                div()
                    .id("controls-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(content),
            )
    }
}

pub fn run_controls_demo(launch: &AppLauncher) {
    run_ios_app(launch, |_window, cx| IosControlsDemo {
        focus_handle: cx.focus_handle(),
        button_tap_count: 0,
        switch_on: false,
        checkbox_checked: false,
        slider_value: 0.5,
        stepper_value: 0,
        text_field_value: String::new(),
        text_field_selection: 0..0,
        text_field_marked_range: None,
        selected_segment: 0,
    });
}

#[derive(Clone, Copy)]
enum DemoKeyboardMode {
    Default,
    Email,
    Url,
    NumberPad,
    DecimalPad,
    Phone,
    OneTimeCode,
    Password,
}

#[derive(Clone, Copy)]
enum DemoReturnMode {
    Submit,
    SubmitAndBlur,
    InsertNewline,
    Search,
    Send,
}

#[derive(Clone, Copy)]
enum DemoAutocorrectMode {
    Default,
    On,
    Off,
}

#[derive(Clone, Copy)]
enum DemoCapitalizationMode {
    None,
    Words,
    Sentences,
    AllCharacters,
}

struct IosTextInputDemo {
    focus_handle: FocusHandle,
    value: String,
    selection: Range<usize>,
    marked_range: Option<Range<usize>>,
    submitted: Vec<String>,
    multiline: bool,
    secure: bool,
    keyboard_mode: DemoKeyboardMode,
    return_mode: DemoReturnMode,
    autocorrect_mode: DemoAutocorrectMode,
    capitalization_mode: DemoCapitalizationMode,
    spell_check: TextInputSpellChecking,
    keyboard_appearance: TextInputKeyboardAppearance,
    soft_keyboard: TextInputSoftKeyboardPolicy,
}

impl Focusable for IosTextInputDemo {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl IosTextInputDemo {
    fn utf16_offset_to_utf8(&self, utf16_offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;

        for ch in self.value.chars() {
            if utf16_count >= utf16_offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }

        utf8_offset
    }

    fn utf8_offset_to_utf16(&self, utf8_offset: usize) -> usize {
        let mut utf8_count = 0;
        let mut utf16_offset = 0;

        for ch in self.value.chars() {
            if utf8_count >= utf8_offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }

        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.utf8_offset_to_utf16(range.start)..self.utf8_offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.utf16_offset_to_utf8(range_utf16.start)..self.utf16_offset_to_utf8(range_utf16.end)
    }

    fn keyboard_type(&self) -> TextInputKeyboardType {
        match self.keyboard_mode {
            DemoKeyboardMode::Default => TextInputKeyboardType::Default,
            DemoKeyboardMode::Email => TextInputKeyboardType::EmailAddress,
            DemoKeyboardMode::Url => TextInputKeyboardType::Url,
            DemoKeyboardMode::NumberPad => TextInputKeyboardType::NumberPad,
            DemoKeyboardMode::DecimalPad => TextInputKeyboardType::DecimalPad,
            DemoKeyboardMode::Phone => TextInputKeyboardType::PhonePad,
            DemoKeyboardMode::OneTimeCode => TextInputKeyboardType::NumberPad,
            DemoKeyboardMode::Password => TextInputKeyboardType::Default,
        }
    }

    fn content_type(&self) -> Option<TextInputContentType> {
        match self.keyboard_mode {
            DemoKeyboardMode::Email => Some(TextInputContentType::EmailAddress),
            DemoKeyboardMode::Url => Some(TextInputContentType::Url),
            DemoKeyboardMode::Phone => Some(TextInputContentType::TelephoneNumber),
            DemoKeyboardMode::OneTimeCode => Some(TextInputContentType::OneTimeCode),
            DemoKeyboardMode::Password => Some(TextInputContentType::Password),
            _ => None,
        }
    }
}

impl EntityInputHandler for IosTextInputDemo {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        adjusted_range.replace(self.range_to_utf16(&range));
        Some(self.value[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selection),
            reversed: false,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let replacement_range = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or(self.marked_range.clone())
            .unwrap_or_else(|| self.selection.clone());

        self.value = format!(
            "{}{}{}",
            &self.value[..replacement_range.start],
            text,
            &self.value[replacement_range.end..]
        );
        let cursor = replacement_range.start + text.len();
        self.selection = cursor..cursor;
        self.marked_range = None;
        cx.notify();
    }

    fn delete_backward(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.marked_range.is_some() || !self.selection.is_empty() {
            self.replace_text_in_range(None, "", window, cx);
            return;
        }

        let cursor = self.selection.end;
        if cursor == 0 {
            return;
        }

        let start = previous_utf8_boundary(&self.value, cursor);
        self.replace_text_in_range(Some(start..cursor), "", window, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let replacement_range = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or(self.marked_range.clone())
            .unwrap_or_else(|| self.selection.clone());

        self.value = format!(
            "{}{}{}",
            &self.value[..replacement_range.start],
            new_text,
            &self.value[replacement_range.end..]
        );

        self.marked_range = if new_text.is_empty() {
            None
        } else {
            Some(replacement_range.start..replacement_range.start + new_text.len())
        };

        self.selection = new_selected_range
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .map(|range| replacement_range.start + range.start..replacement_range.start + range.end)
            .unwrap_or_else(|| {
                let cursor = replacement_range.start + new_text.len();
                cursor..cursor
            });
        cx.notify();
    }

    fn set_selected_text_range(
        &mut self,
        selection: Option<UTF16Selection>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(selection) = selection {
            self.selection = self.range_from_utf16(&selection.range);
            cx.notify();
        }
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        Some(element_bounds)
    }

    fn character_index_for_point(
        &mut self,
        _point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        Some(self.value.len())
    }

    fn text_input_config(&self, _window: &mut Window, _cx: &mut Context<Self>) -> TextInputConfig {
        let return_key_type = match self.return_mode {
            DemoReturnMode::Submit | DemoReturnMode::SubmitAndBlur => TextInputReturnKeyType::Done,
            DemoReturnMode::InsertNewline => TextInputReturnKeyType::Default,
            DemoReturnMode::Search => TextInputReturnKeyType::Search,
            DemoReturnMode::Send => TextInputReturnKeyType::Send,
        };
        let submit_behavior = match self.return_mode {
            DemoReturnMode::Submit | DemoReturnMode::Search | DemoReturnMode::Send => {
                TextInputSubmitBehavior::Submit
            }
            DemoReturnMode::SubmitAndBlur => TextInputSubmitBehavior::SubmitAndBlur,
            DemoReturnMode::InsertNewline => TextInputSubmitBehavior::InsertNewline,
        };
        let autocorrection = match self.autocorrect_mode {
            DemoAutocorrectMode::Default => TextInputAutocorrection::Default,
            DemoAutocorrectMode::On => TextInputAutocorrection::Yes,
            DemoAutocorrectMode::Off => TextInputAutocorrection::No,
        };
        let autocapitalization = match self.capitalization_mode {
            DemoCapitalizationMode::None => TextInputAutocapitalization::None,
            DemoCapitalizationMode::Words => TextInputAutocapitalization::Words,
            DemoCapitalizationMode::Sentences => TextInputAutocapitalization::Sentences,
            DemoCapitalizationMode::AllCharacters => TextInputAutocapitalization::AllCharacters,
        };

        TextInputConfig {
            multiline: self.multiline,
            secure_entry: self.secure,
            keyboard_type: self.keyboard_type(),
            return_key_type,
            text_content_type: self.content_type(),
            autocorrection,
            spell_checking: self.spell_check,
            autocapitalization,
            smart_insert_delete: Some(true),
            keyboard_appearance: self.keyboard_appearance,
            submit_behavior,
            enables_return_key_automatically: false,
            soft_keyboard: self.soft_keyboard,
        }
    }

    fn submit_text_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.submitted.push(self.value.clone());
        if self.submitted.len() > 5 {
            self.submitted.remove(0);
        }
        if matches!(self.return_mode, DemoReturnMode::SubmitAndBlur) {
            window.blur();
        }
        cx.notify();
    }
}

impl Render for IosTextInputDemo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let safe = window.safe_area_insets();
        let focused = self.focus_handle.is_focused(window);
        let display_text = if self.value.is_empty() && !focused {
            "Tap here to type...".to_string()
        } else {
            self.value.clone()
        };
        let input_entity = cx.entity();
        let border_color = if focused { 0x89b4fa } else { 0x585b70 };

        let pill = |label: &str, active: bool| {
            div()
                .px(px(10.0))
                .py(px(6.0))
                .rounded(px(6.0))
                .bg(rgb(if active { 0x89b4fa } else { 0x313244 }))
                .text_color(rgb(if active { 0x1e1e2e } else { 0xcdd6f4 }))
                .child(label.to_string())
        };

        let mut keyboard_modes = div().flex().flex_row().flex_wrap().gap(px(6.0));
        for (label, mode) in [
            ("Default", DemoKeyboardMode::Default),
            ("Email", DemoKeyboardMode::Email),
            ("URL", DemoKeyboardMode::Url),
            ("Number", DemoKeyboardMode::NumberPad),
            ("Decimal", DemoKeyboardMode::DecimalPad),
            ("Phone", DemoKeyboardMode::Phone),
            ("OTP", DemoKeyboardMode::OneTimeCode),
            ("Password", DemoKeyboardMode::Password),
        ] {
            let active =
                std::mem::discriminant(&self.keyboard_mode) == std::mem::discriminant(&mode);
            keyboard_modes = keyboard_modes.child(pill(label, active).on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.keyboard_mode = mode;
                    this.secure = matches!(mode, DemoKeyboardMode::Password);
                    this.multiline = false;
                    cx.notify();
                }),
            ));
        }

        let mut return_modes = div().flex().flex_row().gap(px(6.0)).flex_wrap();
        for (label, mode) in [
            ("Submit", DemoReturnMode::Submit),
            ("Blur", DemoReturnMode::SubmitAndBlur),
            ("Newline", DemoReturnMode::InsertNewline),
            ("Search", DemoReturnMode::Search),
            ("Send", DemoReturnMode::Send),
        ] {
            let active = std::mem::discriminant(&self.return_mode) == std::mem::discriminant(&mode);
            return_modes = return_modes.child(pill(label, active).on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.return_mode = mode;
                    cx.notify();
                }),
            ));
        }

        let mut autocorrect_modes = div().flex().flex_row().gap(px(6.0));
        for (label, mode) in [
            ("Default", DemoAutocorrectMode::Default),
            ("On", DemoAutocorrectMode::On),
            ("Off", DemoAutocorrectMode::Off),
        ] {
            let active =
                std::mem::discriminant(&self.autocorrect_mode) == std::mem::discriminant(&mode);
            autocorrect_modes = autocorrect_modes.child(pill(label, active).on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.autocorrect_mode = mode;
                    cx.notify();
                }),
            ));
        }

        let mut capitalization_modes = div().flex().flex_row().gap(px(6.0)).flex_wrap();
        for (label, mode) in [
            ("None", DemoCapitalizationMode::None),
            ("Words", DemoCapitalizationMode::Words),
            ("Sentences", DemoCapitalizationMode::Sentences),
            ("All", DemoCapitalizationMode::AllCharacters),
        ] {
            let active =
                std::mem::discriminant(&self.capitalization_mode) == std::mem::discriminant(&mode);
            capitalization_modes = capitalization_modes.child(pill(label, active).on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.capitalization_mode = mode;
                    cx.notify();
                }),
            ));
        }

        let mut content = div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .p(px(20.0))
            .pb(safe.bottom)
            .w_full()
            .child(div().text_size(px(24.0)).child("Text Input"))
            .child(div().text_size(px(13.0)).text_color(rgb(0xa6adc8)).child(
                "UIKit-backed GPUI input with selection, IME, traits, submit, and multiline modes.",
            ))
            .child(
                div()
                    .id("text-input-field")
                    .w_full()
                    .min_h(if self.multiline { px(180.0) } else { px(48.0) })
                    .relative()
                    .p(px(12.0))
                    .bg(rgb(0x313244))
                    .rounded(px(8.0))
                    .border_1()
                    .border_color(rgb(border_color))
                    .child(
                        div()
                            .text_size(px(15.0))
                            .text_color(if self.value.is_empty() && !focused {
                                rgb(0x6c7086)
                            } else {
                                rgb(0xcdd6f4)
                            })
                            .child(display_text),
                    )
                    .child(
                        canvas(
                            |_bounds, _window, _cx| (),
                            move |bounds, (), window, cx| {
                                window.handle_input(
                                    &input_entity.read(cx).focus_handle.clone(),
                                    ElementInputHandler::new(bounds, input_entity.clone()),
                                    cx,
                                );
                            },
                        )
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .bottom_0(),
                    )
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_this, _, window, cx| {
                            cx.stop_propagation();
                            cx.focus_self(window);
                            cx.notify();
                        }),
                    ),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(rgb(0xa6adc8))
                    .child(format!(
                        "selection={}..{} marked={:?}",
                        self.selection.start, self.selection.end, self.marked_range
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(8.0))
                    .child(pill("Single line", !self.multiline).on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.multiline = false;
                            cx.notify();
                        }),
                    ))
                    .child(pill("Multiline", self.multiline).on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.multiline = true;
                            this.secure = false;
                            cx.notify();
                        }),
                    ))
                    .child(pill("Secure", self.secure).on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.secure = !this.secure;
                            if this.secure {
                                this.multiline = false;
                                this.keyboard_mode = DemoKeyboardMode::Password;
                            }
                            cx.notify();
                        }),
                    )),
            )
            .child(keyboard_modes)
            .child(return_modes)
            .child(autocorrect_modes)
            .child(capitalization_modes)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(8.0))
                    .child(
                        pill(
                            "Spell Default",
                            matches!(self.spell_check, TextInputSpellChecking::Default),
                        )
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.spell_check = TextInputSpellChecking::Default;
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        pill(
                            "Spell On",
                            matches!(self.spell_check, TextInputSpellChecking::Yes),
                        )
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.spell_check = TextInputSpellChecking::Yes;
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        pill(
                            "Spell Off",
                            matches!(self.spell_check, TextInputSpellChecking::No),
                        )
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.spell_check = TextInputSpellChecking::No;
                                cx.notify();
                            }),
                        ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(8.0))
                    .child(
                        pill(
                            "Keyboard Auto",
                            matches!(self.soft_keyboard, TextInputSoftKeyboardPolicy::Automatic),
                        )
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.soft_keyboard = TextInputSoftKeyboardPolicy::Automatic;
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        pill(
                            "Keyboard Hidden",
                            matches!(self.soft_keyboard, TextInputSoftKeyboardPolicy::Hidden),
                        )
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                this.soft_keyboard = TextInputSoftKeyboardPolicy::Hidden;
                                cx.notify();
                            }),
                        ),
                    ),
            );

        for submitted in self.submitted.iter().rev() {
            content = content.child(
                div()
                    .text_size(px(12.0))
                    .text_color(rgb(0xf9e2af))
                    .child(format!("submitted: {submitted}")),
            );
        }

        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .flex()
            .flex_col()
            .pl(safe.left)
            .pr(safe.right)
            .pt(safe.top)
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_this, _, window, cx| {
                    window.blur();
                    cx.notify();
                }),
            )
            .child(
                div()
                    .id("text-input-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(content),
            )
    }
}

pub fn run_text_input_demo(launch: &AppLauncher) {
    run_ios_app(launch, |_window, cx| IosTextInputDemo {
        focus_handle: cx.focus_handle(),
        value: String::new(),
        selection: 0..0,
        marked_range: None,
        submitted: Vec::new(),
        multiline: false,
        secure: false,
        keyboard_mode: DemoKeyboardMode::Default,
        return_mode: DemoReturnMode::SubmitAndBlur,
        autocorrect_mode: DemoAutocorrectMode::Default,
        capitalization_mode: DemoCapitalizationMode::Sentences,
        spell_check: TextInputSpellChecking::Default,
        keyboard_appearance: TextInputKeyboardAppearance::Default,
        soft_keyboard: TextInputSoftKeyboardPolicy::Automatic,
    });
}

// ---------------------------------------------------------------------------
// 14. Safe Area Demo — visual safe area inset display + opt-out example
// ---------------------------------------------------------------------------

struct IosSafeAreaDemo {
    show_raw: bool,
}

impl Render for IosSafeAreaDemo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let safe = window.safe_area_insets();
        let show_raw = self.show_raw;

        // Visual: show a colored band along each safe area edge
        let inset_label =
            |label: &str, px_val: f32| -> String { format!("{}: {:.0}px", label, px_val) };

        // The outer div fills the full screen — background visible behind notch/home indicator
        div()
            .size_full()
            .bg(rgb(0x1e1e2e))
            .relative()
            // Top safe area band (shows the notch zone)
            .child(
                div()
                    .absolute()
                    .top(px(0.0))
                    .left(px(0.0))
                    .right(px(0.0))
                    .h(safe.top)
                    .bg(rgba(0xcba6f730))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(rgb(0xcba6f7))
                            .child(inset_label("top", f32::from(safe.top))),
                    ),
            )
            // Bottom safe area band (home indicator)
            .child(
                div()
                    .absolute()
                    .bottom(px(0.0))
                    .left(px(0.0))
                    .right(px(0.0))
                    .h(safe.bottom)
                    .bg(rgba(0xf38ba830))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(rgb(0xf38ba8))
                            .child(inset_label("bottom", f32::from(safe.bottom))),
                    ),
            )
            // Left safe area band (landscape notch side)
            .child(
                div()
                    .absolute()
                    .top(safe.top)
                    .bottom(safe.bottom)
                    .left(px(0.0))
                    .w(safe.left)
                    .bg(rgba(0x89b4fa30))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(rgb(0x89b4fa))
                            .child(inset_label("l", f32::from(safe.left))),
                    ),
            )
            // Right safe area band (landscape home indicator side)
            .child(
                div()
                    .absolute()
                    .top(safe.top)
                    .bottom(safe.bottom)
                    .right(px(0.0))
                    .w(safe.right)
                    .bg(rgba(0xa6e3a130))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(rgb(0xa6e3a1))
                            .child(inset_label("r", f32::from(safe.right))),
                    ),
            )
            // Safe content area — the green rectangle shows the actual safe zone
            .child(
                div()
                    .absolute()
                    .top(safe.top)
                    .bottom(safe.bottom)
                    .left(safe.left)
                    .right(safe.right)
                    .border_2()
                    .border_color(rgb(0xa6e3a1))
                    .rounded(px(4.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(12.0))
                    .text_color(rgb(0xcdd6f4))
                    .child(div().text_size(px(18.0)).child("Safe Area Demo"))
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(rgb(0xa6adc8))
                            .child("Colored bands = unsafe zones"),
                    )
                    .child(
                        div()
                            .text_size(px(13.0))
                            .text_color(rgb(0xa6adc8))
                            .child("Green border = safe content area"),
                    )
                    .child(
                        div()
                            .p(px(12.0))
                            .bg(rgb(0x313244))
                            .rounded(px(8.0))
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(rgb(0xcba6f7))
                                    .child(format!("top:    {:.0}px", f32::from(safe.top))),
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(rgb(0xf38ba8))
                                    .child(format!("bottom: {:.0}px", f32::from(safe.bottom))),
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(rgb(0x89b4fa))
                                    .child(format!("left:   {:.0}px", f32::from(safe.left))),
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(rgb(0xa6e3a1))
                                    .child(format!("right:  {:.0}px", f32::from(safe.right))),
                            ),
                    )
                    // Toggle button to demo ignoring safe area vs respecting it
                    .child(
                        div()
                            .id("toggle-safe")
                            .px(px(16.0))
                            .py(px(8.0))
                            .rounded(px(8.0))
                            .bg(if show_raw {
                                rgb(0xf38ba8)
                            } else {
                                rgb(0xa6e3a1)
                            })
                            .text_color(rgb(0x1e1e2e))
                            .text_size(px(13.0))
                            .child(if show_raw {
                                "Mode: Full Screen (unsafe)"
                            } else {
                                "Mode: Safe Area (default)"
                            })
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.show_raw = !this.show_raw;
                                    cx.notify();
                                }),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(0x6c7086))
                            .child("Tap button to toggle safe area mode"),
                    ),
            )
    }
}

pub fn run_safe_area_demo(launch: &AppLauncher) {
    run_ios_app(launch, |_, _| IosSafeAreaDemo { show_raw: false });
}

// ---------------------------------------------------------------------------
// 15. Layout Showcase — comprehensive demo of all GPUI layout APIs
// ---------------------------------------------------------------------------

struct IosLayoutShowcase {
    selected_tab: usize,
}

impl Render for IosLayoutShowcase {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let safe = window.safe_area_insets();
        let selected_tab = self.selected_tab;

        // Tab labels
        let tabs = ["Flex", "Gaps", "Sizing", "Overflow", "Position"];

        // Tab bar at bottom (like a native iOS tab bar)
        let tab_bar = div()
            .w_full()
            .pb(safe.bottom)
            .bg(rgb(0x181825))
            .border_t_1()
            .border_color(rgb(0x313244))
            .flex()
            .flex_row()
            .children(tabs.iter().enumerate().map(|(i, label)| {
                let is_active = selected_tab == i;
                div()
                    .flex_1()
                    .py(px(10.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(2.0))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.selected_tab = i;
                            cx.notify();
                        }),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(if is_active {
                                rgb(0x89b4fa)
                            } else {
                                rgb(0x585b70)
                            })
                            .child(*label),
                    )
            }));

        // Section label helper
        fn section_label(text: &str) -> gpui::Div {
            div()
                .text_size(px(11.0))
                .text_color(rgb(0x6c7086))
                .pb(px(6.0))
                .child(text.to_string())
        }

        // Content for each tab
        let content = match selected_tab {
            // --- Tab 0: Flexbox ---
            0 => div()
                .flex()
                .flex_col()
                .gap(px(16.0))
                .p(px(16.0))
                // Row: flex_row + justify_between
                .child(section_label("flex_row + justify_between"))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .justify_between()
                        .items_center()
                        .child(
                            div()
                                .w(px(60.0))
                                .h(px(40.0))
                                .bg(rgb(0xf38ba8))
                                .rounded(px(6.0)),
                        )
                        .child(
                            div()
                                .w(px(60.0))
                                .h(px(40.0))
                                .bg(rgb(0xa6e3a1))
                                .rounded(px(6.0)),
                        )
                        .child(
                            div()
                                .w(px(60.0))
                                .h(px(40.0))
                                .bg(rgb(0x89b4fa))
                                .rounded(px(6.0)),
                        ),
                )
                // Row: flex_row + justify_center + gap
                .child(section_label("flex_row + justify_center + gap"))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .justify_center()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .w(px(50.0))
                                .h(px(50.0))
                                .bg(rgb(0xfab387))
                                .rounded(px(8.0)),
                        )
                        .child(
                            div()
                                .w(px(50.0))
                                .h(px(50.0))
                                .bg(rgb(0xcba6f7))
                                .rounded(px(8.0)),
                        )
                        .child(
                            div()
                                .w(px(50.0))
                                .h(px(50.0))
                                .bg(rgb(0xf9e2af))
                                .rounded(px(8.0)),
                        ),
                )
                // Column: items_start / center / end
                .child(section_label("flex_col + items_start / center / end"))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex_1()
                                .h(px(80.0))
                                .bg(rgb(0x313244))
                                .rounded(px(6.0))
                                .flex()
                                .flex_col()
                                .items_start()
                                .p(px(4.0))
                                .child(
                                    div()
                                        .w(px(30.0))
                                        .h(px(20.0))
                                        .bg(rgb(0xf38ba8))
                                        .rounded(px(4.0)),
                                )
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .text_color(rgb(0x6c7086))
                                        .child("start"),
                                ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .h(px(80.0))
                                .bg(rgb(0x313244))
                                .rounded(px(6.0))
                                .flex()
                                .flex_col()
                                .items_center()
                                .p(px(4.0))
                                .child(
                                    div()
                                        .w(px(30.0))
                                        .h(px(20.0))
                                        .bg(rgb(0xa6e3a1))
                                        .rounded(px(4.0)),
                                )
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .text_color(rgb(0x6c7086))
                                        .child("center"),
                                ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .h(px(80.0))
                                .bg(rgb(0x313244))
                                .rounded(px(6.0))
                                .flex()
                                .flex_col()
                                .items_end()
                                .p(px(4.0))
                                .child(
                                    div()
                                        .w(px(30.0))
                                        .h(px(20.0))
                                        .bg(rgb(0x89b4fa))
                                        .rounded(px(4.0)),
                                )
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .text_color(rgb(0x6c7086))
                                        .child("end"),
                                ),
                        ),
                )
                // justify_start / center / end / between
                .child(section_label("justify_start / center / end / between"))
                .child(
                    div().w_full().flex().flex_col().gap(px(6.0)).children(
                        [
                            ("start", 0x89b4fau32),
                            ("center", 0xa6e3a1u32),
                            ("end", 0xfab387u32),
                            ("between", 0xcba6f7u32),
                        ]
                        .iter()
                        .map(|(label, color)| {
                            let row = div()
                                .w_full()
                                .h(px(28.0))
                                .bg(rgb(0x313244))
                                .rounded(px(4.0))
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap(px(4.0));
                            let row = match *label {
                                "start" => row.justify_start(),
                                "center" => row.justify_center(),
                                "end" => row.justify_end(),
                                _ => row.justify_between(),
                            };
                            row.child(
                                div()
                                    .w(px(20.0))
                                    .h(px(16.0))
                                    .bg(rgb(*color))
                                    .rounded(px(3.0)),
                            )
                            .child(
                                div()
                                    .w(px(20.0))
                                    .h(px(16.0))
                                    .bg(rgb(*color))
                                    .rounded(px(3.0)),
                            )
                            .child(
                                div()
                                    .text_size(px(9.0))
                                    .text_color(rgb(0x6c7086))
                                    .child(label.to_string()),
                            )
                        }),
                    ),
                )
                .into_any_element(),

            // --- Tab 1: Gaps & Padding ---
            1 => div()
                .flex()
                .flex_col()
                .gap(px(16.0))
                .p(px(16.0))
                // gap() uniform
                .child(section_label("gap(px) — uniform spacing"))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .gap(px(4.0))
                        .children((0..8).map(|_| {
                            div()
                                .flex_1()
                                .h(px(32.0))
                                .bg(rgb(0x89b4fa))
                                .rounded(px(4.0))
                        })),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .gap(px(12.0))
                        .children((0..4).map(|_| {
                            div()
                                .flex_1()
                                .h(px(32.0))
                                .bg(rgb(0xcba6f7))
                                .rounded(px(4.0))
                        })),
                )
                // Padding variants
                .child(section_label("padding — p / px / py / pt / pb / pl / pr"))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child(
                            div()
                                .w_full()
                                .bg(rgb(0x313244))
                                .rounded(px(6.0))
                                .p(px(16.0))
                                .child(
                                    div()
                                        .w_full()
                                        .h(px(20.0))
                                        .bg(rgb(0xf38ba8))
                                        .rounded(px(4.0))
                                        .child(
                                            div()
                                                .text_size(px(9.0))
                                                .text_color(rgb(0x1e1e2e))
                                                .child("p(16)"),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .w_full()
                                .bg(rgb(0x313244))
                                .rounded(px(6.0))
                                .px(px(24.0))
                                .py(px(8.0))
                                .child(
                                    div()
                                        .w_full()
                                        .h(px(20.0))
                                        .bg(rgb(0xa6e3a1))
                                        .rounded(px(4.0))
                                        .child(
                                            div()
                                                .text_size(px(9.0))
                                                .text_color(rgb(0x1e1e2e))
                                                .child("px(24) py(8)"),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .w_full()
                                .bg(rgb(0x313244))
                                .rounded(px(6.0))
                                .pt(px(20.0))
                                .pb(px(4.0))
                                .pl(px(8.0))
                                .pr(px(32.0))
                                .child(
                                    div()
                                        .w_full()
                                        .h(px(20.0))
                                        .bg(rgb(0x89b4fa))
                                        .rounded(px(4.0))
                                        .child(
                                            div()
                                                .text_size(px(9.0))
                                                .text_color(rgb(0x1e1e2e))
                                                .child("pt20 pb4 pl8 pr32"),
                                        ),
                                ),
                        ),
                )
                // Margin variants
                .child(section_label("margin — m / mx / my / mt / mb / ml / mr"))
                .child(
                    div()
                        .w_full()
                        .bg(rgb(0x313244))
                        .rounded(px(6.0))
                        .flex()
                        .flex_row()
                        .child(
                            div()
                                .m(px(8.0))
                                .flex_1()
                                .h(px(36.0))
                                .bg(rgb(0xf9e2af))
                                .rounded(px(4.0))
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .text_color(rgb(0x1e1e2e))
                                        .child("m(8)"),
                                ),
                        )
                        .child(
                            div()
                                .mx(px(4.0))
                                .my(px(12.0))
                                .flex_1()
                                .h(px(36.0))
                                .bg(rgb(0x94e2d5))
                                .rounded(px(4.0))
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .text_color(rgb(0x1e1e2e))
                                        .child("mx4 my12"),
                                ),
                        ),
                )
                .into_any_element(),

            // --- Tab 2: Sizing ---
            2 => div()
                .flex()
                .flex_col()
                .gap(px(16.0))
                .p(px(16.0))
                // Fixed sizes
                .child(section_label("fixed w() / h()"))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child(
                            div()
                                .w(px(100.0))
                                .h(px(24.0))
                                .bg(rgb(0xf38ba8))
                                .rounded(px(4.0))
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .text_color(rgb(0x1e1e2e))
                                        .child("w100 h24"),
                                ),
                        )
                        .child(
                            div()
                                .w(px(200.0))
                                .h(px(32.0))
                                .bg(rgb(0xa6e3a1))
                                .rounded(px(4.0))
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .text_color(rgb(0x1e1e2e))
                                        .child("w200 h32"),
                                ),
                        )
                        .child(
                            div()
                                .w(px(300.0))
                                .h(px(40.0))
                                .bg(rgb(0x89b4fa))
                                .rounded(px(4.0))
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .text_color(rgb(0x1e1e2e))
                                        .child("w300 h40"),
                                ),
                        ),
                )
                // w_full, flex_1, flex_shrink_0
                .child(section_label("w_full / flex_1 / flex_shrink_0"))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_row()
                        .gap(px(6.0))
                        .child(
                            div()
                                .flex_1()
                                .h(px(40.0))
                                .bg(rgb(0xfab387))
                                .rounded(px(4.0))
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .text_color(rgb(0x1e1e2e))
                                        .child("flex_1"),
                                ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .h(px(40.0))
                                .bg(rgb(0xcba6f7))
                                .rounded(px(4.0))
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .text_color(rgb(0x1e1e2e))
                                        .child("flex_1"),
                                ),
                        )
                        .child(
                            div()
                                .flex_shrink_0()
                                .w(px(80.0))
                                .h(px(40.0))
                                .bg(rgb(0xf9e2af))
                                .rounded(px(4.0))
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .text_color(rgb(0x1e1e2e))
                                        .child("shrink_0"),
                                ),
                        ),
                )
                // min/max width & height
                .child(section_label("min_w / max_w / min_h / max_h"))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child(
                            div()
                                .w_full()
                                .bg(rgb(0x313244))
                                .rounded(px(6.0))
                                .p(px(8.0))
                                .child(
                                    div()
                                        .min_w(px(80.0))
                                        .max_w(px(200.0))
                                        .h(px(28.0))
                                        .bg(rgb(0x94e2d5))
                                        .rounded(px(4.0))
                                        .child(
                                            div()
                                                .text_size(px(9.0))
                                                .text_color(rgb(0x1e1e2e))
                                                .child("min80 max200"),
                                        ),
                                ),
                        )
                        .child(
                            div()
                                .w_full()
                                .bg(rgb(0x313244))
                                .rounded(px(6.0))
                                .p(px(8.0))
                                .flex()
                                .flex_row()
                                .gap(px(4.0))
                                .children((0..5).map(|i| {
                                    div()
                                        .flex_1()
                                        .min_h(px(20.0))
                                        .max_h(px(60.0))
                                        .h(px(20.0 + i as f32 * 10.0))
                                        .bg(rgb(0x89b4fa))
                                        .rounded(px(4.0))
                                })),
                        ),
                )
                .into_any_element(),

            // --- Tab 3: Overflow ---
            3 => {
                let scroll_colors = [
                    0xf38ba8u32,
                    0xa6e3a1,
                    0x89b4fa,
                    0xfab387,
                    0xcba6f7,
                    0xf9e2af,
                    0x94e2d5,
                ];

                let mut vlist = div().flex().flex_col().gap(px(6.0));
                for i in 0..20 {
                    vlist = vlist.child(
                        div()
                            .w_full()
                            .h(px(44.0))
                            .bg(rgb(scroll_colors[i % scroll_colors.len()]))
                            .rounded(px(6.0))
                            .flex()
                            .items_center()
                            .px(px(12.0))
                            .text_color(rgb(0x1e1e2e))
                            .child(format!("overflow_y row {}", i + 1)),
                    );
                }

                let mut hstrip = div().flex().flex_row().gap(px(8.0));
                for i in 0..15 {
                    hstrip = hstrip.child(
                        div()
                            .w(px(100.0))
                            .h(px(80.0))
                            .flex_shrink_0()
                            .bg(rgb(scroll_colors[i % scroll_colors.len()]))
                            .rounded(px(8.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(rgb(0x1e1e2e))
                            .child(format!("{}", i + 1)),
                    );
                }

                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .p(px(16.0))
                    .child(section_label("overflow_y_scroll"))
                    .child(
                        div()
                            .id("overflow-y-demo")
                            .w_full()
                            .h(px(200.0))
                            .overflow_y_scroll()
                            .bg(rgb(0x181825))
                            .rounded(px(8.0))
                            .p(px(8.0))
                            .child(vlist),
                    )
                    .child(section_label("overflow_x_scroll"))
                    .child(
                        div()
                            .id("overflow-x-demo")
                            .w_full()
                            .overflow_x_scroll()
                            .bg(rgb(0x181825))
                            .rounded(px(8.0))
                            .p(px(8.0))
                            .child(hstrip),
                    )
                    .child(section_label("overflow_hidden (clips content)"))
                    .child(
                        div()
                            .w(px(200.0))
                            .h(px(60.0))
                            .overflow_hidden()
                            .bg(rgb(0x313244))
                            .rounded(px(8.0))
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .w(px(400.0))
                                    .h(px(40.0))
                                    .bg(rgb(0xf38ba8))
                                    .flex()
                                    .items_center()
                                    .px(px(8.0))
                                    .child(
                                        "This text is wider than the container and gets clipped",
                                    ),
                            ),
                    )
                    .into_any_element()
            }

            // --- Tab 4: Position ---
            _ => div()
                .flex()
                .flex_col()
                .gap(px(16.0))
                .p(px(16.0))
                .child(section_label("relative + absolute positioning"))
                .child(
                    div()
                        .w_full()
                        .h(px(180.0))
                        .bg(rgb(0x313244))
                        .rounded(px(8.0))
                        .relative()
                        // Corners
                        .child(
                            div()
                                .absolute()
                                .top(px(8.0))
                                .left(px(8.0))
                                .w(px(48.0))
                                .h(px(48.0))
                                .bg(rgb(0xf38ba8))
                                .rounded(px(6.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(8.0))
                                .text_color(rgb(0x1e1e2e))
                                .child("t8 l8"),
                        )
                        .child(
                            div()
                                .absolute()
                                .top(px(8.0))
                                .right(px(8.0))
                                .w(px(48.0))
                                .h(px(48.0))
                                .bg(rgb(0xa6e3a1))
                                .rounded(px(6.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(8.0))
                                .text_color(rgb(0x1e1e2e))
                                .child("t8 r8"),
                        )
                        .child(
                            div()
                                .absolute()
                                .bottom(px(8.0))
                                .left(px(8.0))
                                .w(px(48.0))
                                .h(px(48.0))
                                .bg(rgb(0x89b4fa))
                                .rounded(px(6.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(8.0))
                                .text_color(rgb(0x1e1e2e))
                                .child("b8 l8"),
                        )
                        .child(
                            div()
                                .absolute()
                                .bottom(px(8.0))
                                .right(px(8.0))
                                .w(px(48.0))
                                .h(px(48.0))
                                .bg(rgb(0xfab387))
                                .rounded(px(6.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(8.0))
                                .text_color(rgb(0x1e1e2e))
                                .child("b8 r8"),
                        )
                        // Center element
                        .child(
                            div()
                                .absolute()
                                .inset(px(60.0))
                                .bg(rgb(0xcba6f7))
                                .rounded(px(8.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(10.0))
                                .text_color(rgb(0x1e1e2e))
                                .child("inset(60)"),
                        ),
                )
                .child(section_label("z-index layering (absolute stacked divs)"))
                .child(
                    div()
                        .w_full()
                        .h(px(120.0))
                        .relative()
                        .child(
                            div()
                                .absolute()
                                .top(px(0.0))
                                .left(px(0.0))
                                .w(px(120.0))
                                .h(px(80.0))
                                .bg(rgb(0xf38ba8))
                                .rounded(px(8.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(10.0))
                                .text_color(rgb(0x1e1e2e))
                                .child("layer 1"),
                        )
                        .child(
                            div()
                                .absolute()
                                .top(px(20.0))
                                .left(px(40.0))
                                .w(px(120.0))
                                .h(px(80.0))
                                .bg(rgb(0xa6e3a1))
                                .rounded(px(8.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(10.0))
                                .text_color(rgb(0x1e1e2e))
                                .child("layer 2"),
                        )
                        .child(
                            div()
                                .absolute()
                                .top(px(40.0))
                                .left(px(80.0))
                                .w(px(120.0))
                                .h(px(80.0))
                                .bg(rgb(0x89b4fa))
                                .rounded(px(8.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(10.0))
                                .text_color(rgb(0x1e1e2e))
                                .child("layer 3"),
                        ),
                )
                .into_any_element(),
        };

        // Full layout: header + scrollable tab content + tab bar
        div()
            .size_full()
            .flex()
            .flex_col()
            .pl(safe.left)
            .pr(safe.right)
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            // Header (notch-aware)
            .child(
                div()
                    .w_full()
                    .pt(safe.top)
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_end()
                    .bg(rgb(0x181825))
                    .child(
                        div()
                            .h(px(52.0))
                            .flex()
                            .items_center()
                            .text_size(px(17.0))
                            .text_color(rgb(0xcdd6f4))
                            .child(format!("Layout — {}", tabs[selected_tab])),
                    ),
            )
            // Scrollable content area
            .child(
                div()
                    .id("layout-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(content),
            )
            // Tab bar (home indicator-aware)
            .child(tab_bar)
    }
}

pub fn run_layout_showcase(launch: &AppLauncher) {
    run_ios_app(launch, |_, _| IosLayoutShowcase { selected_tab: 0 });
}

// ---------------------------------------------------------------------------
// Validation Demos
// ---------------------------------------------------------------------------

struct IosTier1FilePickerDemo {
    status: String,
    picked_paths: Vec<String>,
    save_path: Option<String>,
}

impl IosTier1FilePickerDemo {
    fn open_files(&mut self, cx: &mut Context<Self>) {
        self.status = "Opening file picker (files)...".to_string();
        cx.notify();

        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Open Files".into()),
        });

        cx.spawn(async move |this, cx| {
            let result = rx.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(Ok(Some(paths))) => {
                        this.picked_paths = paths
                            .into_iter()
                            .map(|path| path.display().to_string())
                            .collect();
                        this.status = format!("Selected {} file(s)", this.picked_paths.len());
                    }
                    Ok(Ok(None)) => {
                        this.status = "File picker cancelled".to_string();
                    }
                    Ok(Err(err)) => {
                        this.status = format!("File picker failed: {err}");
                    }
                    Err(err) => {
                        this.status = format!("File picker channel failed: {err}");
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn open_directories(&mut self, cx: &mut Context<Self>) {
        self.status = "Opening file picker (directories)...".to_string();
        cx.notify();

        let rx = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: true,
            prompt: Some("Open Folders".into()),
        });

        cx.spawn(async move |this, cx| {
            let result = rx.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(Ok(Some(paths))) => {
                        this.picked_paths = paths
                            .into_iter()
                            .map(|path| path.display().to_string())
                            .collect();
                        this.status = format!("Selected {} folder(s)", this.picked_paths.len());
                    }
                    Ok(Ok(None)) => {
                        this.status = "Directory picker cancelled".to_string();
                    }
                    Ok(Err(err)) => {
                        this.status = format!("Directory picker failed: {err}");
                    }
                    Err(err) => {
                        this.status = format!("Directory picker channel failed: {err}");
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn save_file(&mut self, cx: &mut Context<Self>) {
        self.status = "Opening save picker...".to_string();
        cx.notify();

        let default_dir = std::env::temp_dir();
        let rx = cx.prompt_for_new_path(&default_dir, Some("gpui-tier1-save.txt"));
        cx.spawn(async move |this, cx| {
            let result = rx.await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(Ok(Some(path))) => {
                        let path = path.display().to_string();
                        this.save_path = Some(path.clone());
                        this.status = "Save destination selected".to_string();
                        this.picked_paths = vec![path];
                    }
                    Ok(Ok(None)) => {
                        this.status = "Save picker cancelled".to_string();
                    }
                    Ok(Err(err)) => {
                        this.status = format!("Save picker failed: {err}");
                    }
                    Err(err) => {
                        this.status = format!("Save picker channel failed: {err}");
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

impl Render for IosTier1FilePickerDemo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let safe = window.safe_area_insets();

        let mut picked = div().flex().flex_col().gap(px(4.0));
        if self.picked_paths.is_empty() {
            picked = picked.child(
                div()
                    .text_size(px(12.0))
                    .text_color(rgb(0xa6adc8))
                    .child("No selected paths yet"),
            );
        } else {
            for path in self.picked_paths.iter().take(8) {
                picked = picked.child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(0xa6adc8))
                        .child(path.clone()),
                );
            }
        }

        div()
            .id("tier1-file-picker-root")
            .size_full()
            .pt(safe.top + px(16.0))
            .pb(safe.bottom + px(16.0))
            .pl(safe.left + px(16.0))
            .pr(safe.right + px(16.0))
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(div().text_size(px(22.0)).child("File Open/Save Validation"))
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(rgb(0xa6adc8))
                    .child("Uses real UIDocumentPicker open/export flows."),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(10.0))
                    .child(
                        action_button(
                            "tier1-pick-files",
                            "Open Files",
                            cx.listener(|this, _event, _window, cx| this.open_files(cx)),
                        ),
                    )
                    .child(
                        action_button(
                            "tier1-pick-folders",
                            "Open Folders",
                            cx.listener(|this, _event, _window, cx| this.open_directories(cx)),
                        ),
                    )
                    .child(
                        action_button(
                            "tier1-save-file",
                            "Save As",
                            cx.listener(|this, _event, _window, cx| this.save_file(cx)),
                        ),
                    ),
            )
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(rgb(0xf9e2af))
                    .child(format!("Status: {}", self.status)),
            )
            .child(
                div()
                    .w_full()
                    .p(px(10.0))
                    .rounded(px(8.0))
                    .bg(rgb(0x181825))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(rgb(0xcdd6f4))
                            .child("Selected paths"),
                    )
                    .child(picked),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(rgb(0xa6adc8))
                    .child(format!(
                        "Last save path: {}",
                        self.save_path.as_deref().unwrap_or("<none>")
                    )),
            )
    }
}

pub fn run_file_picker_demo(launch: &AppLauncher) {
    run_ios_app(launch, |_window, _cx| IosTier1FilePickerDemo {
        status: "Ready".to_string(),
        picked_paths: Vec::new(),
        save_path: None,
    });
}

struct IosTier1ClipboardDemo {
    status: String,
    last_text: String,
    last_metadata: String,
    last_image_size: usize,
}

impl IosTier1ClipboardDemo {
    fn copy_text_with_metadata(&mut self, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string_with_metadata(
            "Tier1 clipboard text".to_string(),
            "{\"source\":\"ios-demo\",\"kind\":\"text+metadata\"}".to_string(),
        ));
        self.status = "Wrote text + metadata to clipboard".to_string();
        cx.notify();
    }

    fn copy_image(&mut self, cx: &mut Context<Self>) {
        let image = Image::from_bytes(ImageFormat::Png, IOS_CLIPBOARD_TEST_PNG.to_vec());
        cx.write_to_clipboard(ClipboardItem::new_image(&image));
        self.status = format!(
            "Wrote PNG image to clipboard ({} bytes)",
            image.bytes().len()
        );
        cx.notify();
    }

    fn paste(&mut self, cx: &mut Context<Self>) {
        let Some(item) = cx.read_from_clipboard() else {
            self.status = "Clipboard was empty".to_string();
            self.last_text = "<none>".to_string();
            self.last_metadata = "<none>".to_string();
            self.last_image_size = 0;
            cx.notify();
            return;
        };

        self.last_text = item.text().unwrap_or_else(|| "<none>".to_string());
        self.last_metadata = item
            .metadata()
            .cloned()
            .unwrap_or_else(|| "<none>".to_string());
        self.last_image_size = item
            .entries()
            .iter()
            .find_map(|entry| match entry {
                ClipboardEntry::Image(image) => Some(image.bytes().len()),
                _ => None,
            })
            .unwrap_or(0);
        let count = item.entries().len();
        self.status = format!(
            "Read {} clipboard {}",
            count,
            if count == 1 { "entry" } else { "entries" }
        );
        cx.notify();
    }
}

impl Render for IosTier1ClipboardDemo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let safe = window.safe_area_insets();

        div()
            .id("tier1-clipboard-root")
            .size_full()
            .pt(safe.top + px(16.0))
            .pb(safe.bottom + px(16.0))
            .pl(safe.left + px(16.0))
            .pr(safe.right + px(16.0))
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(div().text_size(px(22.0)).child("Clipboard Validation"))
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(rgb(0xa6adc8))
                    .child("Verifies text+metadata and image clipboard read/write paths."),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(10.0))
                    .child(
                        action_button(
                            "tier1-copy-text-meta",
                            "Copy Text+Meta",
                            cx.listener(|this, _event, _window, cx| {
                                this.copy_text_with_metadata(cx)
                            }),
                        ),
                    )
                    .child(
                        action_button(
                            "tier1-copy-image",
                            "Copy Image",
                            cx.listener(|this, _event, _window, cx| this.copy_image(cx)),
                        ),
                    )
                    .child(
                        action_button(
                            "tier1-paste",
                            "Paste",
                            cx.listener(|this, _event, _window, cx| this.paste(cx)),
                        ),
                    ),
            )
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(rgb(0xf9e2af))
                    .child(format!("Status: {}", self.status)),
            )
            .child(
                div()
                    .w_full()
                    .p(px(10.0))
                    .rounded(px(8.0))
                    .bg(rgb(0x181825))
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(rgb(0xa6adc8))
                            .child(format!("Text: {}", self.last_text)),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(rgb(0xa6adc8))
                            .child(format!("Metadata: {}", self.last_metadata)),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(rgb(0xa6adc8))
                            .child(format!("Image bytes: {}", self.last_image_size)),
                    ),
            )
    }
}

pub fn run_clipboard_demo(launch: &AppLauncher) {
    run_ios_app(launch, |_window, _cx| IosTier1ClipboardDemo {
        status: "Ready".to_string(),
        last_text: "<none>".to_string(),
        last_metadata: "<none>".to_string(),
        last_image_size: 0,
    });
}

struct IosTier1FileDropDemo {
    status: String,
    hover_count: usize,
    dropped_paths: Vec<String>,
}

impl Render for IosTier1FileDropDemo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let safe = window.safe_area_insets();
        let border_color = if self.hover_count > 0 {
            0xa6e3a1
        } else {
            0x585b70
        };

        let mut dropped = div().flex().flex_col().gap(px(4.0));
        if self.dropped_paths.is_empty() {
            dropped = dropped.child(
                div()
                    .text_size(px(12.0))
                    .text_color(rgb(0xa6adc8))
                    .child("No dropped paths yet"),
            );
        } else {
            for path in self.dropped_paths.iter().take(8) {
                dropped = dropped.child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(0xa6adc8))
                        .child(path.clone()),
                );
            }
        }

        div()
            .id("tier1-file-drop-root")
            .size_full()
            .pt(safe.top + px(16.0))
            .pb(safe.bottom + px(16.0))
            .pl(safe.left + px(16.0))
            .pr(safe.right + px(16.0))
            .bg(rgb(0x1e1e2e))
            .text_color(rgb(0xcdd6f4))
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(
                div()
                    .text_size(px(22.0))
                    .child("External File Drop Validation"),
            )
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(rgb(0xa6adc8))
                    .child("Drag files from Files app into the drop zone."),
            )
            .child(
                div()
                    .id("tier1-drop-zone")
                    .w_full()
                    .h(px(180.0))
                    .p(px(12.0))
                    .bg(rgb(0x313244))
                    .rounded(px(10.0))
                    .border_2()
                    .border_color(rgb(border_color))
                    .can_drop(|value, _, _| value.is::<ExternalPaths>())
                    .on_drag_move(cx.listener(
                        |this, event: &gpui::DragMoveEvent<ExternalPaths>, _window, cx| {
                            let paths = event.drag(cx).paths();
                            this.hover_count = paths.len();
                            this.status = format!("Dragging {} file(s)...", this.hover_count);
                            cx.notify();
                        },
                    ))
                    .on_drop(cx.listener(|this, paths: &ExternalPaths, _window, cx| {
                        this.hover_count = 0;
                        this.dropped_paths = paths
                            .paths()
                            .iter()
                            .map(|path| path.display().to_string())
                            .collect();
                        this.status = format!("Dropped {} file(s)", this.dropped_paths.len());
                        cx.notify();
                    }))
                    .child(div().text_size(px(14.0)).child("Drop zone")),
            )
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(rgb(0xf9e2af))
                    .child(format!("Status: {}", self.status)),
            )
            .child(
                div()
                    .w_full()
                    .p(px(10.0))
                    .rounded(px(8.0))
                    .bg(rgb(0x181825))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(rgb(0xcdd6f4))
                            .child("Dropped paths"),
                    )
                    .child(dropped),
            )
    }
}

pub fn run_file_drop_demo(launch: &AppLauncher) {
    run_ios_app(launch, |_window, _cx| IosTier1FileDropDemo {
        status: "Waiting for drag".to_string(),
        hover_count: 0,
        dropped_paths: Vec::new(),
    });
}

// ---------------------------------------------------------------------------
// Demo dispatcher — called from ObjC main.m with demo name as C string
// ---------------------------------------------------------------------------

pub fn available_demo_names() -> impl Iterator<Item = &'static str> {
    IOS_DEMOS.iter().map(|demo| demo.name)
}

pub fn run_demo_named<'a>(name: &'a str, launch: &AppLauncher) -> Result<(), UnknownIosDemo<'a>> {
    match name {
        "hello_world" => {
            run_hello_world(launch);
            Ok(())
        }
        "touch" => {
            run_touch_demo(launch);
            Ok(())
        }
        "text" => {
            run_text_demo(launch);
            Ok(())
        }
        "lifecycle" => {
            run_lifecycle_demo(launch);
            Ok(())
        }
        "combined" => {
            run_combined_demo(launch);
            Ok(())
        }
        "scroll" => {
            run_scroll_demo(launch);
            Ok(())
        }
        "vertical_scroll" => {
            run_vertical_scroll_demo(launch);
            Ok(())
        }
        "horizontal_scroll" => {
            run_horizontal_scroll_demo(launch);
            Ok(())
        }
        "pinch" => {
            run_pinch_demo(launch);
            Ok(())
        }
        "rotation" => {
            run_rotation_demo(launch);
            Ok(())
        }
        "text_input" => {
            run_text_input_demo(launch);
            Ok(())
        }
        "controls" => {
            run_controls_demo(launch);
            Ok(())
        }
        "safe_area" => {
            run_safe_area_demo(launch);
            Ok(())
        }
        "layout_showcase" => {
            run_layout_showcase(launch);
            Ok(())
        }
        "file_picker" => {
            run_file_picker_demo(launch);
            Ok(())
        }
        "clipboard" => {
            run_clipboard_demo(launch);
            Ok(())
        }
        "file_drop" => {
            run_file_drop_demo(launch);
            Ok(())
        }
        _ => Err(UnknownIosDemo { name }),
    }
}
