use std::{
    cell::RefCell,
    ops::Deref,
    rc::Rc,
    sync::{Arc, Mutex},
    thread,
};

use crate::html_parser::{HtmlItem, HtmlLink, HtmlLoader, HtmlText, TeleText, MIDDLE_TEXT_MAX_LEN};
use egui::{FontId, InputState, RichText, TextStyle};

use super::common::input_to_num;

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

struct TelePage {
    page: i32,
    sub_page: i32,
}

impl TelePage {
    fn new(page: i32, sub_page: i32) -> Self {
        Self { page, sub_page }
    }

    fn from_page_str(page: &str) -> Self {
        let current_page = page[0..3].parse::<i32>().unwrap();
        let sub_page = page[4..8].parse::<i32>().unwrap();

        Self::new(current_page, sub_page)
    }

    fn to_page_str(&self) -> String {
        format!("{}_{:04}.htm", self.page, self.sub_page)
    }
}

struct GuiYleText<'a> {
    ui: &'a mut egui::Ui,
    ctx: Rc<RefCell<&'a mut GuiYleTextContext>>,
    panel_width: f32,
    char_width: f32,
    is_small: bool,
}

impl<'a> GuiYleText<'a> {
    fn get_page_str(&self) -> String {
        let page_buf = &self.ctx.borrow().page_buffer;
        let page_num = if !page_buf.is_empty() {
            let mut page_str = "---".as_bytes().to_vec();
            for (idx, num) in page_buf.iter().enumerate() {
                page_str[idx] = b'0' + (*num as u8);
            }
            String::from_utf8(page_str.to_vec()).unwrap()
        } else {
            self.ctx.borrow().current_page.page.to_string()
        };

        format!("P{}", page_num)
    }

    fn draw_header_small(&mut self, title: &HtmlText) {
        // align with page navigation
        let chw = self.char_width;
        let page_len = chw * 4.0;
        let title_len = (title.chars().count() as f32) * chw;
        let title_space = (self.panel_width / 2.0) - (title_len / 2.0) - page_len;
        let page = self.get_page_str();

        self.ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label(page);
            ui.add_space(title_space);
            ui.label(title.clone());
        });
    }

    fn draw_header_normal(&mut self, title: &HtmlText) {
        // align with page navigation
        let chw = self.char_width;
        let nav_length = chw * 69.0;
        let nav_start = (self.panel_width / 2.0) - (nav_length / 2.0);
        let page_len = chw * 4.0;
        let time_len = chw * 15.0;
        let title_len = (title.chars().count() as f32) * chw;

        let title_space = (nav_length / 2.0) - (title_len / 2.0) - page_len;
        let time_space = nav_length - title_space - page_len - title_len - time_len;

        let page = self.get_page_str();

        self.ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(nav_start);
            ui.label(page);
            ui.add_space(title_space);
            ui.label(title.clone());
            ui.add_space(time_space);
            let now = chrono::Utc::now();
            ui.label(now.format("%d.%m. %H:%M:%S").to_string());
        });
    }

    fn draw_header(&mut self, title: &HtmlText) {
        if self.is_small {
            self.draw_header_small(title);
        } else {
            self.draw_header_normal(title);
        }
    }

    fn draw_page_navigation_small(&mut self, navigation: &[HtmlItem]) {
        let mut body_font = TextStyle::Body.resolve(self.ui.style());
        body_font.size *= 3.0;
        let arrow_width = self.ui.fonts().glyph_width(&body_font, 'W');
        let chars_len = arrow_width * 4.0 + self.char_width * 9.0;
        let page_nav_start = (self.panel_width / 2.0) - (chars_len / 2.0);
        let ctx = &self.ctx;
        self.ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(page_nav_start);
            for (idx, item) in navigation.iter().enumerate() {
                let icon = match idx {
                    0 => "←",
                    1 => "↑",
                    2 => "↓",
                    3 => "→",
                    _ => "?",
                };

                let icon_text = RichText::new(icon).font(FontId::monospace(body_font.size));
                match item {
                    HtmlItem::Link(link) => {
                        if ui.link(icon_text).clicked() {
                            ctx.borrow_mut().load_page(&link.url);
                        };
                    }
                    HtmlItem::Text(_) => {
                        ui.label(icon_text);
                    }
                }

                if idx < 3 {
                    ui.label(" | ");
                }
            }
        });
    }

    fn draw_page_navigation_normal(&mut self, navigation: &[HtmlItem]) {
        // "Edellinen sivu | Edellinen alasivu | Seuraava alasivu | Seuraava sivu" is 69 char
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

    fn draw_page_navigation(&mut self, navigation: &[HtmlItem]) {
        if self.is_small {
            self.draw_page_navigation_small(navigation);
        } else {
            self.draw_page_navigation_normal(navigation);
        }
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

    fn draw_bottom_navigation_small(&mut self, navigation: &[HtmlLink]) {
        // "Teksti-TV" is 9 chars
        let page_nav_start = (self.panel_width / 2.0) - (self.char_width * 9.0 / 2.0);
        let ctx = &self.ctx;
        self.ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(page_nav_start);
            let link = navigation.last().unwrap();
            link.add_to_ui(ui, ctx.clone());
        });
    }

    fn draw_bottom_navigation_normal(&mut self, navigation: &[HtmlLink]) {
        // "Kotimaa | Ulkomaat | Talous | Urheilu | Svenska sidor | Teksti-TV" is 65 chars
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

    fn draw_bottom_navigation(&mut self, navigation: &[HtmlLink]) {
        if self.is_small {
            self.draw_bottom_navigation_small(navigation);
        } else {
            self.draw_bottom_navigation_normal(navigation);
        }
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
        // Aligned with page navigation
        let nav_len = char_width * 69.0;
        let is_small = nav_len + 4.0 > panel_width;

        Self {
            ui,
            ctx: Rc::new(RefCell::new(ctx)),
            char_width,
            panel_width,
            is_small,
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
    current_page: TelePage,
    page_buffer: Vec<i32>,
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
            current_page: TelePage::new(100, 1),
            page_buffer: Vec::with_capacity(3),
        }
    }

    pub fn handle_input(&mut self, input: &InputState) {
        if let Some(num) = input_to_num(input) {
            if self.page_buffer.len() < 3 {
                self.page_buffer.push(num);
            }

            if self.page_buffer.len() == 3 {
                let page_num = self.page_buffer.iter().fold(0, |acum, val| acum * 10 + val);
                self.current_page = TelePage::new(page_num, 1);
                self.page_buffer.clear();
                self.load_current_page();
            }
        }
    }

    pub fn draw(&mut self, ui: &mut egui::Ui) {
        GuiYleText::new(ui, self).draw();
    }

    pub fn load_current_page(&mut self) {
        // TODO: sub_pages
        let page = self.current_page.to_page_str();
        self.load_page(&page);
    }

    pub fn load_page(&mut self, page: &str) {
        let ctx = self.egui.clone();
        let state = self.state.clone();
        self.current_page = TelePage::from_page_str(page);
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
