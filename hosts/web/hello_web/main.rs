fn main() {
    gpui_platform::web_init();
    gpui_examples::web::run_hello_web(&|setup| {
        let app = gpui_platform::application();
        app.run(move |cx| setup(cx));
    });
}
