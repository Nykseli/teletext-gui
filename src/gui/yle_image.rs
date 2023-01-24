use std::{cell::RefCell, ops::Deref, rc::Rc};

use egui::{FontId, InputState, RichText, TextStyle};
use egui_extras::RetainedImage;

use crate::parser::{HtmlLink, HtmlText, YleImage};

use super::common::{FetchState, GuiContext, IGuiCtx, PageDraw, TelePage, TelePager};

pub struct GuiYleImage<'a> {
    ui: &'a mut egui::Ui,
    ctx: Rc<RefCell<&'a mut GuiContext<YleImage>>>,
    panel_width: f32,
    char_width: f32,
    is_small: bool,
}

impl<'a> GuiYleImage<'a> {
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
        let title_space = self.panel_width - title_len - page_len;
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
        let title = format!("{} YLE TEKSTI-TV", title);
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
            let now = chrono::Local::now();
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

    fn draw_image(&mut self, image: &[u8]) {
        self.ui
            .with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                let image = RetainedImage::from_image_bytes("debug_name", image).unwrap();
                image.show_max_size(ui, ui.available_size());
            });
    }

    fn draw_page_navigation_small(&mut self, navigation: &[Option<HtmlLink>]) {
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
                    Some(link) => {
                        if ui.link(icon_text).clicked() {
                            ctx.borrow_mut().load_page(&link.url, true);
                        };
                    }
                    None => {
                        ui.label(icon_text);
                    }
                }

                if idx < 3 {
                    ui.label(" | ");
                }
            }
        });
    }

    fn draw_page_navigation_normal(&mut self, navigation: &[Option<HtmlLink>]) {
        // "Edellinen sivu | Edellinen alasivu | Seuraava alasivu | Seuraava sivu" is 69 char
        let valid_link: Vec<&HtmlLink> = navigation.iter().filter_map(|n| n.as_ref()).collect();
        let text_len = valid_link.iter().fold(0.0, |acum, val| {
            acum + val.inner_text.chars().count() as f32
        });
        let page_nav_start = (self.panel_width / 2.0) - (self.char_width * text_len / 2.0);
        let ctx = &self.ctx;
        self.ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(page_nav_start);
            for (idx, item) in valid_link.iter().enumerate() {
                item.add_to_ui(ui, ctx.clone());
                if idx < valid_link.len() - 1 {
                    ui.label(" | ");
                }
            }
        });
    }

    fn draw_page_navigation(&mut self, navigation: &[Option<HtmlLink>]) {
        if self.is_small {
            self.draw_page_navigation_small(navigation);
        } else {
            self.draw_page_navigation_normal(navigation);
        }
    }
}

impl<'a> PageDraw<'a, YleImage> for GuiYleImage<'a> {
    fn draw(&mut self) {
        let ctx = &self.ctx;
        let state = self.ctx.borrow().state.clone();

        match state.lock().unwrap().deref() {
            FetchState::Complete(page) => {
                self.draw_header(&page.title);
                self.draw_image(&page.image);
                self.draw_page_navigation(&page.botton_navigation);
            }
            FetchState::Fetching => {
                self.ui
                    .with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        ui.label("Loading...");
                    });
            }
            FetchState::Error => {
                self.ui
                    .with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        ui.label("Load failed...");
                        if ui.link("Return to previous page").clicked() {
                            ctx.borrow_mut().return_from_error_page();
                        }
                    });
            }
            FetchState::InitFailed => {
                self.ui
                    .with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        ui.label("Load failed...");
                        if ui.link("Try again").clicked() {
                            ctx.borrow_mut().load_current_page();
                        }
                    });
            }
            FetchState::Init => {
                self.ui
                    .with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        ui.label("Opening...");
                    });
                ctx.borrow_mut().load_current_page();
            }
        };
    }

    fn new(ui: &'a mut egui::Ui, ctx: &'a mut GuiContext<YleImage>) -> Self {
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

pub struct GuiYleImageContext {
    ctx: GuiContext<YleImage>,
}

impl GuiYleImageContext {
    pub fn new(ctx: GuiContext<YleImage>) -> Self {
        Self { ctx }
    }
}

impl IGuiCtx for GuiYleImageContext {
    fn handle_input(&mut self, input: &InputState) {
        self.ctx.handle_input(input)
    }

    fn draw(&mut self, ui: &mut egui::Ui) {
        self.ctx.draw(ui);
        GuiYleImage::new(ui, &mut self.ctx).draw();
    }

    fn set_refresh_interval(&mut self, interval: u64) {
        self.ctx.set_refresh_interval(interval)
    }

    fn stop_refresh_interval(&mut self) {
        self.ctx.stop_refresh_interval()
    }

    fn return_from_error_page(&mut self) {
        self.ctx.return_from_error_page()
    }

    fn load_current_page(&mut self) {
        self.ctx.load_current_page()
    }

    fn load_page(&mut self, page: &str, add_to_history: bool) {
        self.ctx.load_page(page, add_to_history)
    }
}

impl TelePager for YleImage {
    fn to_full_page(page: &TelePage) -> String {
        // https://yle.fi/aihe/yle-ttv/json?P=100_0001
        format!(
            "https://yle.fi/aihe/yle-ttv/json?P={}_{:04}",
            page.page, page.sub_page
        )
    }

    fn to_page_str(page: &TelePage) -> String {
        format!("{}_{:04}", page.page, page.sub_page)
    }

    fn from_page_str(page: &str) -> TelePage {
        let current_page = page[0..3].parse::<i32>().unwrap();
        let sub_page = page[4..8].parse::<i32>().unwrap();

        TelePage::new(current_page, sub_page)
    }
}
