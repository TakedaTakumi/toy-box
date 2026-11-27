//! 上のメニューバー。

use egui::Ui;

use super::{PanelActions, PanelLayout};
use crate::settings::{DisplaySettings, Shading};

pub fn show(ui: &mut Ui, settings: &mut DisplaySettings, layout: &mut PanelLayout) -> PanelActions {
    let mut actions = PanelActions::default();
    egui::MenuBar::new().ui(ui, |ui| {
        ui.menu_button("表示", |ui| {
            ui.checkbox(&mut layout.tools_open, "ツールパネル");
            ui.checkbox(&mut layout.properties_open, "プロパティパネル");
            ui.separator();
            for shading in Shading::ALL {
                ui.radio_value(&mut settings.shading, shading, shading.label());
            }
            ui.separator();
            ui.checkbox(&mut settings.show_surface, "曲面");
            ui.checkbox(&mut settings.show_cage, "ケージ");
            ui.separator();
            if ui.button("視点をリセット").clicked() {
                actions.reset_camera = true;
                ui.close();
            }
        });
        ui.menu_button("ヘルプ", |ui| {
            ui.label(crate::APP_TITLE);
            ui.label(concat!("バージョン ", env!("CARGO_PKG_VERSION")));
            ui.weak("操作説明は、ツールパネルの「操作説明」にあります");
            ui.separator();
            ui.label("フォントのライセンス");
            ui.weak("Gregory UI Sans（SIL OFL 1.1）");
            ui.weak("文書：LICENSE、LICENSE_Hack、");
            ui.weak("LICENSE_GenJyuuGothic");
            #[cfg(target_arch = "wasm32")]
            ui.hyperlink_to("LICENSE を開く", "fonts/LICENSE");
        });
    });
    actions
}
