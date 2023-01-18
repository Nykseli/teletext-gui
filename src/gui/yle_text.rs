use std::{
    cell::RefCell,
    ops::Deref,
    rc::Rc,
    sync::{Arc, Mutex},
    thread,
};

use crate::html_parser::{HtmlItem, HtmlLink, HtmlLoader, HtmlText, TeleText, MIDDLE_TEXT_MAX_LEN};
use egui::TextStyle;

impl HtmlItem {
    fn add_to_ui(&self, ui: &mut egui::Ui, ctx: Rc<RefCell<&mut GuiYleTextContext>>) {
        match self {
            HtmlItem::Link(link) => {
                link.add_to_ui(ui, ctx);
            }
            HtmlItem::Text(text) => {
                ui.label(text);
            }
        }
    }
}

impl HtmlLink {
    fn add_to_ui(&self, ui: &mut egui::Ui, ctx: Rc<RefCell<&mut GuiYleTextContext>>) {
        if ui.link(&self.inner_text).clicked() {
            println!("Clicked {}", self.url);
            ctx.borrow_mut().load_page(&self.url);
        }
    }
}

struct GuiYleText<'a> {
    ui: &'a mut egui::Ui,
    ctx: Rc<RefCell<&'a mut GuiYleTextContext>>,
    panel_width: f32,
    char_width: f32,
}

impl<'a> GuiYleText<'a> {
    fn draw_header(&mut self, title: &HtmlText) {
        // align with page navigation
        let chw = self.char_width;
        let nav_length = chw * 69.0;
        let nav_start = (self.panel_width / 2.0) - (nav_length / 2.0);
        let page_len = chw * 4.0;
        let time_len = chw * 15.0;
        let title_len = (title.chars().count() as f32) * chw;

        let title_space = (nav_length / 2.0) - (title_len / 2.0) - page_len;
        let time_space = nav_length - title_space - page_len - title_len - time_len;

        self.ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(nav_start);
            // TODO: print current page
            ui.label("P100");
            ui.add_space(title_space);
            ui.label(title.clone());
            ui.add_space(time_space);
            let now = chrono::Utc::now();
            ui.label(now.format("%d.%m. %H:%M:%S").to_string());
        });
    }

    fn draw_page_navigation(&mut self, navigation: &[HtmlItem]) {
        // "Edellinen sivu | Edellinen alasivu | Seuraava alasivu | Seuraava sivu" is 69 chars
        let page_nav_start = (self.panel_width / 2.0) - (self.char_width * 69.0 / 2.0);
        let ctx = &self.ctx;
        self.ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(page_nav_start);
            for (idx, item) in navigation.iter().enumerate() {
                item.add_to_ui(ui, ctx.clone());
                if idx < 3 {
                    ui.label(" | ");
                }
            }
        });
    }

    fn draw_middle(&mut self, rows: &Vec<Vec<HtmlItem>>) {
        let middle_text_start =
            (self.panel_width / 2.0) - (self.char_width * (MIDDLE_TEXT_MAX_LEN as f32) / 2.0);
        let ctx = &self.ctx;
        for row in rows {
            self.ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.add_space(middle_text_start);
                for item in row {
                    item.add_to_ui(ui, ctx.clone());
                }
            });
        }
    }

    fn draw_sub_pages(&mut self, pages: &[HtmlItem]) {
        let middle_text_start =
            (self.panel_width / 2.0) - (self.char_width * (MIDDLE_TEXT_MAX_LEN as f32) / 2.0);
        let ctx = &self.ctx;
        self.ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(middle_text_start);
            for item in pages {
                item.add_to_ui(ui, ctx.clone());
            }
        });
    }

    fn draw_bottom_navigation(&mut self, navigation: &[HtmlLink]) {
        // "Kotimaa | Ulkomaat | Talous | Urheilu | Svenska sidor | Teksti-TV" is 88 chars
        let page_nav_start = (self.panel_width / 2.0) - (self.char_width * 65.0 / 2.0);
        let ctx = &self.ctx;
        self.ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(page_nav_start);
            for (idx, item) in navigation.iter().enumerate() {
                item.add_to_ui(ui, ctx.clone());
                if idx < 5 {
                    ui.label(" | ");
                }
            }
        });
    }

    pub fn draw(&mut self) {
        let state = self.ctx.borrow().state.clone();

        // TODO: draw all states
        if let FetchState::Complete(page) = state.lock().unwrap().deref() {
            self.draw_header(&page.title);
            self.draw_page_navigation(&page.page_navigation);
            self.draw_middle(&page.middle_rows);
            self.draw_sub_pages(&page.sub_pages);
            self.ui.label("\n");
            self.draw_page_navigation(&page.page_navigation);
            self.draw_bottom_navigation(&page.bottom_navigation);
        };
    }

    pub fn new(ui: &'a mut egui::Ui, ctx: &'a mut GuiYleTextContext) -> Self {
        let panel_width = ui.available_width();
        let body_font = TextStyle::Body.resolve(ui.style());
        let char_width = ui.fonts().glyph_width(&body_font, 'W');

        Self {
            ui,
            ctx: Rc::new(RefCell::new(ctx)),
            char_width,
            panel_width,
        }
    }
}

enum FetchState {
    /// No fetch has been done, so the state is uninitialised
    // Uninit,
    // Fetching,
    // TODO: error codes
    // Error,
    Complete(TeleText),
}

pub struct GuiYleTextContext {
    egui: egui::Context,
    state: Arc<Mutex<FetchState>>,
}

impl GuiYleTextContext {
    pub fn new(egui: egui::Context) -> Self {
        // Load test page
        let file = "101.htm";
        let pobj = HtmlLoader::new(file);
        let mut parser = TeleText::new();
        parser.parse(pobj).unwrap();

        Self {
            egui,
            state: Arc::new(Mutex::new(FetchState::Complete(parser))),
        }
    }

    pub fn draw(&mut self, ui: &mut egui::Ui) {
        GuiYleText::new(ui, self).draw();
    }

    pub fn load_page(&mut self, page: &str) {
        let ctx = self.egui.clone();
        let state = self.state.clone();
        let page = page.to_string();

        thread::spawn(move || {
            let site = &format!("https://yle.fi/tekstitv/txt/{}", page);
            log::info!("Load page: {}", site);
            let body = match reqwest::blocking::get(site) {
                Err(err) => panic!("{:#?}", err),
                Ok(body) => body,
            };
            let body = match body.text() {
                Err(err) => panic!("{:#?}", err),
                Ok(body) => body,
            };
            let mut parser = TeleText::new();
            parser.parse(HtmlLoader { page_data: body }).unwrap();
            *state.lock().unwrap() = FetchState::Complete(parser);
            ctx.request_repaint();
        });
    }
}
