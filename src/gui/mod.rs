use std::time::Duration;

use crate::html_parser::{HtmlLoader, TeleText};

mod yle_text;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TeleTextApp {
    // TODO: Result with actual error information like failed download etc
    #[serde(skip)]
    page_text: Option<TeleText>,
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

        // Load test page
        let file = "101.htm";
        let pobj = HtmlLoader::new(file);
        let mut parser = TeleText::new();
        parser.parse(pobj).unwrap();

        Self {
            page_text: Some(parser),
        }
    }
}

impl eframe::App for TeleTextApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let Self { page_text } = self;
        let page = page_text.as_ref().unwrap();

        egui::CentralPanel::default().show(ctx, |ui| {
            yle_text::GuiYleText::new(ui).draw(page);
        });

        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
