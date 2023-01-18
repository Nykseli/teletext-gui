use std::time::Duration;

mod yle_text;
use self::yle_text::GuiYleTextContext;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TeleTextApp {
    #[serde(skip)]
    page: Option<GuiYleTextContext>,
}

impl TeleTextApp {
    /// Called once before the first frame.
    pub fn new(ctx: &eframe::CreationContext<'_>) -> Self {
        // Override default fonts with our own font
        let mut fonts = egui::FontDefinitions::empty();
        fonts.font_data.insert(
            "default_font".to_owned(),
            egui::FontData::from_static(include_bytes!("../../DroidSansMono.ttf")),
        );

        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "default_font".to_owned());

        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("default_font".to_owned());

        ctx.egui_ctx.set_fonts(fonts);

        Self {
            page: Some(GuiYleTextContext::new(ctx.egui_ctx.clone())),
        }
    }
}

impl eframe::App for TeleTextApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let Self { page } = self;

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(page) = page {
                page.draw(ui);
            }
        });

        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
