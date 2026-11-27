//! ネイティブはウィンドウで、web は canvas で起動する。

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    use gregory_editor::{APP_TITLE, DEPTH_BUFFER_BITS, GregoryEditorApp};

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(APP_TITLE)
            .with_inner_size([1200.0, 760.0])
            .with_min_inner_size([360.0, 320.0]),
        depth_buffer: DEPTH_BUFFER_BITS,
        ..Default::default()
    };
    eframe::run_native(
        APP_TITLE,
        options,
        Box::new(|cc| Ok(Box::new(GregoryEditorApp::new(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    web::start();
}

#[cfg(target_arch = "wasm32")]
mod web {
    use eframe::wasm_bindgen::JsCast as _;
    use gregory_editor::{DEPTH_BUFFER_BITS, GregoryEditorApp};
    use web_sys::{Document, HtmlCanvasElement};

    const CANVAS_ID: &str = "the_canvas_id";
    const LOADING_ID: &str = "loading_text";
    const PANIC_ID: &str = "panic_notice";

    pub fn start() {
        eframe::WebLogger::init(log::LevelFilter::Info).ok();
        wasm_bindgen_futures::spawn_local(async {
            let Some(document) = web_sys::window().and_then(|w| w.document()) else {
                log::error!("document を取得できませんでした");
                return;
            };
            let Some(canvas) = document
                .get_element_by_id(CANVAS_ID)
                .and_then(|e| e.dyn_into::<HtmlCanvasElement>().ok())
            else {
                show_failure(
                    &document,
                    &format!("canvas（id: {CANVAS_ID}）が見つかりません"),
                );
                return;
            };
            let options = eframe::WebOptions {
                depth_buffer: DEPTH_BUFFER_BITS,
                ..Default::default()
            };
            let runner = eframe::WebRunner::new();
            install_panic_notice();
            let result = runner
                .start(
                    canvas,
                    options,
                    Box::new(|cc| Ok(Box::new(GregoryEditorApp::new(cc)))),
                )
                .await;
            match result {
                Ok(()) => {
                    if let Some(loading) = document.get_element_by_id(LOADING_ID) {
                        loading.remove();
                    }
                }
                Err(error) => show_failure(&document, &format!("{error:?}")),
            }
        });
    }

    /// 起動後の panic を画面に出す。eframe が入れたフックの後に呼ぶこと。
    fn install_panic_notice() {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            previous(info);
            show_panic_notice();
        }));
    }

    fn show_panic_notice() {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        if document.get_element_by_id(PANIC_ID).is_some() {
            return;
        }
        let (Ok(notice), Some(root)) = (document.create_element("p"), document.document_element())
        else {
            return;
        };
        notice.set_id(PANIC_ID);
        notice.set_text_content(Some(
            "エラーが起きました。ページを再読み込みしてください（詳細はコンソール）",
        ));
        let style = "position:fixed;top:50%;left:50%;transform:translate(-50%,-50%);max-width:calc(100% - 32px);padding:12px 16px;background:#1b1b1b;color:#ff8080;border:1px solid #ff8080;font:14px/1.6 system-ui,sans-serif;text-align:center";
        notice.set_attribute("style", style).ok();
        root.append_child(&notice).ok();
    }

    /// 起動できなかった理由を、loading 表示の場所に出す。
    fn show_failure(document: &Document, detail: &str) {
        log::error!("起動できませんでした：{detail}");
        if let Some(loading) = document.get_element_by_id(LOADING_ID) {
            loading.set_text_content(Some(&format!(
                "起動できませんでした。WebGPU か WebGL2 に対応したブラウザで開いてください。（{detail}）"
            )));
        }
    }
}
