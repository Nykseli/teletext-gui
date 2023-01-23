use std::{
    cell::RefCell,
    ops::Deref,
    rc::Rc,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use crate::parser::{
    HtmlItem, HtmlLink, HtmlLoader, HtmlParser, HtmlText, TeleText, MIDDLE_TEXT_MAX_LEN,
};
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
            ctx.borrow_mut().load_page(&self.url, true);
        }
    }
}

#[derive(Clone, Copy)]
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

    fn to_page_str(self) -> String {
        format!("{}_{:04}.htm", self.page, self.sub_page)
    }
}

struct TeleHistory {
    pages: Vec<TelePage>,
    current: usize,
}

impl TeleHistory {
    fn new(first_page: TelePage) -> Self {
        Self {
            pages: vec![first_page],
            current: 0,
        }
    }

    /// Trucks current history to the current page
    fn add(&mut self, page: TelePage) {
        self.current += 1;
        self.pages.truncate(self.current);
        self.pages.push(page);
    }

    fn prev(&mut self) -> Option<TelePage> {
        if self.current > 0 {
            self.current -= 1;
            return Some(*self.pages.get(self.current).unwrap());
        }

        None
    }

    // Go to previous page and truncate the current history
    fn prev_trunc(&mut self) -> Option<TelePage> {
        if self.current > 0 {
            self.pages.truncate(self.current);
            self.current -= 1;
            return Some(*self.pages.get(self.current).unwrap());
        }

        None
    }

    fn next(&mut self) -> Option<TelePage> {
        if self.current < self.pages.len() - 1 {
            self.current += 1;
            return Some(*self.pages.get(self.current).unwrap());
        }

        None
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
                            ctx.borrow_mut().load_page(&link.url, true);
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
        let ctx = &self.ctx;
        let state = self.ctx.borrow().state.clone();

        // TODO: draw all states
        match state.lock().unwrap().deref() {
            FetchState::Complete(page) => {
                self.draw_header(&page.title);
                self.draw_page_navigation(&page.page_navigation);
                self.draw_middle(&page.middle_rows);
                self.draw_sub_pages(&page.sub_pages);
                self.ui.label("\n");
                self.draw_page_navigation(&page.page_navigation);
                self.draw_bottom_navigation(&page.bottom_navigation);
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
    Init,
    InitFailed,
    Fetching,
    // TODO: error codes
    Error,
    Complete(TeleText),
}

pub struct GuiYleTextContext {
    egui: egui::Context,
    state: Arc<Mutex<FetchState>>,
    current_page: TelePage,
    history: TeleHistory,
    page_buffer: Vec<i32>,
    worker: Option<YleTextGuiWorker>,
}

impl GuiYleTextContext {
    pub fn new(egui: egui::Context) -> Self {
        let current_page = TelePage::new(100, 1);

        Self {
            egui,
            current_page,
            state: Arc::new(Mutex::new(FetchState::Init)),
            page_buffer: Vec::with_capacity(3),
            history: TeleHistory::new(current_page),
            worker: None,
        }
    }

    pub fn handle_input(&mut self, input: &InputState) {
        // Ignore input while fetching
        match *self.state.lock().unwrap() {
            FetchState::Complete(_) => {}
            _ => return,
        };

        if let Some(num) = input_to_num(input) {
            if self.page_buffer.len() < 3 {
                self.page_buffer.push(num);
            }

            if self.page_buffer.len() == 3 {
                let page_num = self.page_buffer.iter().fold(0, |acum, val| acum * 10 + val);
                self.page_buffer.clear();
                self.load_page(&TelePage::new(page_num, 1).to_page_str(), true);
            }
        }

        // prev
        if input.pointer.button_released(egui::PointerButton::Extra1) {
            if let Some(page) = self.history.prev() {
                self.current_page = page;
                self.load_current_page();
            }
        }

        // next
        if input.pointer.button_released(egui::PointerButton::Extra2) {
            if let Some(page) = self.history.next() {
                self.current_page = page;
                self.load_current_page();
            }
        }
    }

    pub fn draw(&mut self, ui: &mut egui::Ui) {
        if let Some(worker) = &mut self.worker {
            if worker.should_refresh() {
                worker.use_refresh();
                self.load_current_page();
            }
        }

        GuiYleText::new(ui, self).draw();
    }

    pub fn set_refresh_interval(&mut self, interval: u64) {
        if let Some(worker) = &mut self.worker {
            worker.set_interval(interval);
        } else {
            let mut worker = YleTextGuiWorker::new(interval);
            worker.start();
            self.worker = Some(worker);
        }
    }

    pub fn stop_refresh_interval(&mut self) {
        self.worker = None;
    }

    pub fn return_from_error_page(&mut self) {
        if let Some(page) = self.history.prev_trunc() {
            self.current_page = page;
            self.load_current_page();
        }
    }

    pub fn load_current_page(&mut self) {
        let page = self.current_page.to_page_str();
        self.load_page(&page, false);
    }

    pub fn load_page(&mut self, page: &str, add_to_history: bool) {
        let ctx = self.egui.clone();
        let state = self.state.clone();
        let page = page.to_string();

        self.current_page = TelePage::from_page_str(&page);
        if add_to_history {
            self.history.add(self.current_page)
        }

        thread::spawn(move || {
            let is_init = matches!(
                *state.lock().unwrap(),
                FetchState::Init | FetchState::InitFailed
            );

            *state.lock().unwrap() = FetchState::Fetching;
            let site = &format!("https://yle.fi/tekstitv/txt/{}", page);
            log::info!("Load page: {}", site);
            let new_state = match Self::fetch_page(site) {
                Ok(parser) => FetchState::Complete(parser),
                Err(_) => {
                    if is_init {
                        FetchState::InitFailed
                    } else {
                        FetchState::Error
                    }
                }
            };

            *state.lock().unwrap() = new_state;
            ctx.request_repaint();
        });
    }

    fn fetch_page(site: &str) -> Result<TeleText, ()> {
        let body = reqwest::blocking::get(site).map_err(|_| ())?;
        let body = body.text().map_err(|_| ())?;
        let teletext = TeleText::new()
            .parse(HtmlLoader { page_data: body })
            .map_err(|_| ())?;
        Ok(teletext)
    }
}

pub struct YleTextGuiWorker {
    running: Arc<Mutex<bool>>,
    timer: Arc<Mutex<u64>>,
    /// How often refresh should happen in seconds
    interval: Arc<Mutex<u64>>,
    should_refresh: Arc<Mutex<bool>>,
}

impl YleTextGuiWorker {
    pub fn new(interval: u64) -> Self {
        Self {
            running: Arc::new(Mutex::new(false)),
            should_refresh: Arc::new(Mutex::new(false)),
            timer: Arc::new(Mutex::new(0)),
            interval: Arc::new(Mutex::new(interval)),
        }
    }

    pub fn start(&mut self) {
        *self.running.lock().unwrap() = true;
        let running = self.running.clone();
        let timer = self.timer.clone();
        let interval = self.interval.clone();
        let should_refresh = self.should_refresh.clone();
        thread::spawn(move || {
            while *running.lock().unwrap() {
                thread::sleep(Duration::from_secs(1));
                let mut refresh = should_refresh.lock().unwrap();
                // Only incerement timeres when there's no refresh happening
                if !*refresh {
                    let mut timer = timer.lock().unwrap();
                    let new_time = *timer + 1;
                    let interval = *interval.lock().unwrap();
                    if new_time >= interval {
                        *timer = 0;
                        *refresh = true;
                    } else {
                        *timer = new_time;
                    }
                }
            }
        });
    }

    pub fn stop(&mut self) {
        *self.timer.lock().unwrap() = 0;
        *self.running.lock().unwrap() = false;
    }

    pub fn set_interval(&mut self, interval: u64) {
        *self.timer.lock().unwrap() = 0;
        *self.interval.lock().unwrap() = interval;
    }

    pub fn should_refresh(&self) -> bool {
        *self.should_refresh.lock().unwrap()
    }

    pub fn use_refresh(&mut self) {
        *self.should_refresh.lock().unwrap() = false;
    }
}

impl Drop for YleTextGuiWorker {
    fn drop(&mut self) {
        self.stop();
    }
}

impl Default for YleTextGuiWorker {
    fn default() -> Self {
        // 5 minutes
        Self::new(300)
    }
}
