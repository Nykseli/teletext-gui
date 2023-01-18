use crate::html_parser::{HtmlItem, HtmlText, TeleText, MIDDLE_TEXT_MAX_LEN};
use egui::TextStyle;

impl HtmlItem {
    fn add_to_ui(&self, ui: &mut egui::Ui) {
        match self {
            HtmlItem::Link(link) => {
                if ui.link(&link.inner_text).clicked() {
                    println!("Clicked {}", link.url);
                }
            }
            HtmlItem::Text(text) => {
                ui.label(text);
            }
        }
    }
}

pub struct GuiYleText<'a> {
    ui: &'a mut egui::Ui,
    panel_width: f32,
    char_width: f32,
}

impl<'a> GuiYleText<'a> {
    fn draw_header(&mut self, title: &HtmlText) {
        self.ui
            .with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.label(title.clone());
            });
    }

    fn draw_page_navigation(&mut self, navigation: &[HtmlItem]) {
        // "Edellinen sivu | Edellinen alasivu | Seuraava alasivu | Seuraava sivu" is 69 chars
        let page_nav_start = (self.panel_width / 2.0) - (self.char_width * 69.0 / 2.0);
        self.ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(page_nav_start);
            for (idx, item) in navigation.iter().enumerate() {
                item.add_to_ui(ui);
                if idx < 3 {
                    ui.label(" | ");
                }
            }
        });
    }

    fn draw_middle(&mut self, rows: &Vec<Vec<HtmlItem>>) {
        let middle_text_start =
            (self.panel_width / 2.0) - (self.char_width * (MIDDLE_TEXT_MAX_LEN as f32) / 2.0);
        for row in rows {
            self.ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.add_space(middle_text_start);
                for item in row {
                    item.add_to_ui(ui)
                }
            });
        }
    }

    pub fn draw(&mut self, page: &TeleText) {
        self.draw_header(&page.title);
        self.draw_page_navigation(&page.page_navigation);
        self.draw_middle(&page.middle_rows);
        self.ui.label("\n");
        self.draw_page_navigation(&page.page_navigation);
    }

    pub fn new(ui: &'a mut egui::Ui) -> Self {
        let panel_width = ui.available_width();
        let body_font = TextStyle::Body.resolve(ui.style());
        let char_width = ui.fonts().glyph_width(&body_font, 'W');

        Self {
            ui,
            char_width,
            panel_width,
        }
    }
}
