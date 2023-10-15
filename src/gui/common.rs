use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

#[cfg(not(target_arch = "wasm32"))]
use std::{thread, time::Duration};

use egui::{self, InputState, Key::*, PointerState};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

use crate::parser::{HtmlItem, HtmlLink, HtmlLoader, HtmlParser};

const NUM_KEYS: [egui::Key; 10] = [Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9];

/// Return None if number is not pressed
pub fn input_to_num(input: &InputState) -> Option<i32> {
    for (idx, key) in NUM_KEYS.iter().enumerate() {
        if input.key_released(*key) {
            return Some(idx as i32);
        }
    }

    None
}

pub trait PageDraw<'a, T: HtmlParser + TelePager + Send + 'static> {
    fn draw(&mut self);
    fn new(ui: &'a mut egui::Ui, ctx: &'a mut GuiContext<T>) -> Self;
}

pub trait TelePager {
    fn to_full_page(page: &TelePage) -> String;
    fn to_page_str(page: &TelePage) -> String;
    fn from_page_str(page: &str) -> TelePage;
}

#[derive(Clone, Copy)]
pub struct TelePage {
    pub page: i32,
    pub sub_page: i32,
}

impl TelePage {
    pub fn new(page: i32, sub_page: i32) -> Self {
        Self { page, sub_page }
    }
}

pub struct TeleHistory {
    pages: Vec<TelePage>,
    current: usize,
}

impl TeleHistory {
    pub fn new(first_page: TelePage) -> Self {
        Self {
            pages: vec![first_page],
            current: 0,
        }
    }

    /// Trucks current history to the current page
    pub fn add(&mut self, page: TelePage) {
        self.current += 1;
        self.pages.truncate(self.current);
        self.pages.push(page);
    }

    pub fn prev(&mut self) -> Option<TelePage> {
        if self.current > 0 {
            self.current -= 1;
            return Some(*self.pages.get(self.current).unwrap());
        }

        None
    }

    // Go to previous page and truncate the current history
    pub fn prev_trunc(&mut self) -> Option<TelePage> {
        if self.current > 0 {
            self.pages.truncate(self.current);
            self.current -= 1;
            return Some(*self.pages.get(self.current).unwrap());
        }

        None
    }

    pub fn next(&mut self) -> Option<TelePage> {
        if self.current < self.pages.len() - 1 {
            self.current += 1;
            return Some(*self.pages.get(self.current).unwrap());
        }

        None
    }
}

pub struct GuiWorker {
    #[cfg(not(target_arch = "wasm32"))]
    running: Arc<Mutex<bool>>,
    #[cfg(not(target_arch = "wasm32"))]
    timer: Arc<Mutex<u64>>,
    /// How often refresh should happen in seconds
    interval: Arc<Mutex<u64>>,
    should_refresh: Arc<Mutex<bool>>,
    #[cfg(target_arch = "wasm32")]
    interval_handle: Option<i32>,
}

impl GuiWorker {
    pub fn new(interval: u64) -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            running: Arc::new(Mutex::new(false)),
            #[cfg(not(target_arch = "wasm32"))]
            timer: Arc::new(Mutex::new(0)),
            should_refresh: Arc::new(Mutex::new(false)),
            interval: Arc::new(Mutex::new(interval)),
            #[cfg(target_arch = "wasm32")]
            interval_handle: None,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
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

    #[cfg(target_arch = "wasm32")]
    pub fn start(&mut self) {
        let interval = self.interval.clone();
        let should_refresh = self.should_refresh.clone();

        let interval_fn = Some(Closure::wrap(Box::new(move || {
            let mut refresh = should_refresh.lock().unwrap();
            *refresh = true;
        }) as Box<dyn FnMut()>));

        let handle = web_sys::window()
            .expect("no global `window` exists")
            .set_interval_with_callback_and_timeout_and_arguments_0(
                interval_fn.as_ref().unwrap().as_ref().unchecked_ref(),
                (*interval.lock().unwrap() as i32) * 1000,
            )
            .unwrap();
        // clone, forget avoids Uncaught Error: closure invoked recursively or after being dropped
        // https://stackoverflow.com/a/53219594
        // https://rustwasm.github.io/wasm-bindgen/examples/closures.html
        // FIXME: fix the memory leak
        interval_fn.unwrap().forget();
        self.interval_handle = Some(handle);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn stop(&mut self) {
        *self.timer.lock().unwrap() = 0;
        *self.running.lock().unwrap() = false;
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(interval) = self.interval_handle {
                web_sys::window()
                    .expect("no global `window` exists")
                    .clear_interval_with_handle(interval);
            }
            self.interval_handle = None;
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn stop(&mut self) {
        if let Some(interval) = self.interval_handle {
            web_sys::window()
                .expect("no global `window` exists")
                .clear_interval_with_handle(interval);
        }
        self.interval_handle = None;
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_interval(&mut self, interval: u64) {
        *self.timer.lock().unwrap() = 0;
        *self.interval.lock().unwrap() = interval;
    }

    #[cfg(target_arch = "wasm32")]
    pub fn set_interval(&mut self, interval: u64) {
        *self.interval.lock().unwrap() = interval;
        self.stop();
        self.start();
    }

    pub fn should_refresh(&self) -> bool {
        *self.should_refresh.lock().unwrap()
    }

    pub fn use_refresh(&mut self) {
        *self.should_refresh.lock().unwrap() = false;
    }
}

impl Drop for GuiWorker {
    fn drop(&mut self) {
        self.stop();
    }
}

impl Default for GuiWorker {
    fn default() -> Self {
        // 5 minutes
        Self::new(300)
    }
}

pub enum FetchState<T: HtmlParser> {
    /// No fetch has been done, so the state is uninitialised
    Init,
    InitFailed,
    Fetching,
    // TODO: error codes
    Error,
    Complete(T),
}

pub trait IGuiCtx {
    fn handle_input(&mut self, input: InputState);
    fn draw(&mut self, ui: &mut egui::Ui);
    fn set_refresh_interval(&mut self, interval: u64);
    fn stop_refresh_interval(&mut self);
    fn return_from_error_page(&mut self);
    fn load_current_page(&mut self);
    fn load_page(&mut self, page: &str, add_to_history: bool);
}

pub struct GuiContext<T: HtmlParser + TelePager + Send + 'static> {
    pub egui: egui::Context,
    pub state: Arc<Mutex<FetchState<T>>>,
    pub current_page: TelePage,
    pub history: TeleHistory,
    pub page_buffer: Vec<i32>,
    pub worker: Option<GuiWorker>,
    pub pointer: PointerState,
}

impl<T: HtmlParser + TelePager + Send + 'static> GuiContext<T> {
    pub fn new(egui: egui::Context) -> Self {
        let current_page = TelePage::new(100, 1);

        Self {
            egui,
            current_page,
            state: Arc::new(Mutex::new(FetchState::Init)),
            page_buffer: Vec::with_capacity(3),
            history: TeleHistory::new(current_page),
            worker: None,
            pointer: Default::default(),
        }
    }

    /// Used for testing/dev only
    #[allow(dead_code)]
    pub fn from_file(egui: egui::Context, file: &str) -> Self {
        let current_page = TelePage::new(100, 1);
        let pobj = HtmlLoader::new(file);
        let parser = T::new();
        let completed = parser.parse(pobj).unwrap();

        Self {
            egui,
            current_page,
            state: Arc::new(Mutex::new(FetchState::Complete(completed))),
            page_buffer: Vec::with_capacity(3),
            history: TeleHistory::new(current_page),
            worker: None,
            pointer: Default::default(),
        }
    }

    /// Helper function to load GuiContext without having to relyi on http request
    /// Example:
    /// ```rust
    /// let page_string = String::from_utf8(include_bytes!("../../100.htm").to_vec()).unwrap();
    /// let mut page = Box::new(GuiYleTextContext::new(GuiContext::from_string(
    ///     ctx.egui_ctx.clone(),
    ///     &page_string,
    /// ))) as Box<dyn IGuiCtx>;
    /// let page_ref = &mut page as &mut Box<dyn IGuiCtx>;
    /// ```
    #[allow(dead_code)]
    pub fn from_string(egui: egui::Context, src: &str) -> Self {
        let current_page = TelePage::new(100, 1);
        let pobj = HtmlLoader {
            page_data: src.into(),
        };
        let parser = T::new();
        let completed = parser.parse(pobj).unwrap();

        Self {
            egui,
            current_page,
            state: Arc::new(Mutex::new(FetchState::Complete(completed))),
            page_buffer: Vec::with_capacity(3),
            history: TeleHistory::new(current_page),
            worker: None,
            pointer: Default::default(),
        }
    }

    pub fn handle_input(&mut self, input: InputState) {
        // Ignore input while fetching
        match *self.state.lock().unwrap() {
            FetchState::Complete(_) => {}
            _ => return,
        };

        if let Some(num) = input_to_num(&input) {
            if self.page_buffer.len() < 3 {
                self.page_buffer.push(num);
            }

            if self.page_buffer.len() == 3 {
                let page_num = self.page_buffer.iter().fold(0, |acum, val| acum * 10 + val);
                self.page_buffer.clear();
                self.load_page(&T::to_page_str(&TelePage::new(page_num, 1)), true);
            }
        }

        // After keyboard stuff is handled, move the ownership of pointer to self and
        // deal with mouse inputs
        self.pointer = input.pointer;
        // prev
        if self.pointer.button_released(egui::PointerButton::Extra1) {
            if let Some(page) = self.history.prev() {
                self.current_page = page;
                self.load_current_page();
            }
        }

        // next
        if self.pointer.button_released(egui::PointerButton::Extra2) {
            if let Some(page) = self.history.next() {
                self.current_page = page;
                self.load_current_page();
            }
        }
    }

    pub fn draw(&mut self, _ui: &mut egui::Ui) {
        if let Some(worker) = &mut self.worker {
            if worker.should_refresh() {
                worker.use_refresh();
                self.load_current_page();
            }
        }
    }

    pub fn set_refresh_interval(&mut self, interval: u64) {
        if let Some(worker) = &mut self.worker {
            worker.set_interval(interval);
        } else {
            let mut worker = GuiWorker::new(interval);
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
        let page = T::to_page_str(&self.current_page);
        self.load_page(&page, false);
    }

    pub fn load_page(&mut self, page: &str, add_to_history: bool) {
        let ctx = self.egui.clone();
        let state = self.state.clone();
        let page = T::from_page_str(page);

        self.current_page = page;
        if add_to_history {
            self.history.add(self.current_page)
        }

        #[cfg(not(target_arch = "wasm32"))]
        thread::spawn(move || {
            let is_init = matches!(
                *state.lock().unwrap(),
                FetchState::Init | FetchState::InitFailed
            );

            *state.lock().unwrap() = FetchState::Fetching;
            let site = &T::to_full_page(&page);
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

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            let is_init = matches!(
                *state.lock().unwrap(),
                FetchState::Init | FetchState::InitFailed
            );

            *state.lock().unwrap() = FetchState::Fetching;
            let site = &T::to_full_page(&page);
            tracing::info!("Load page: {}", site);
            let fetched = Self::fetch_page(site).await;
            let new_state = match fetched {
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

    #[cfg(not(target_arch = "wasm32"))]
    fn fetch_page(site: &str) -> Result<T, ()> {
        use reqwest::header::{HeaderMap, HeaderValue};

        // let body = reqwest::blocking::get(site).unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", HeaderValue::from_static("curl/7.81.0"));
        let body = reqwest::blocking::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap()
            .get(site)
            .send()
            .unwrap();
        let body = body.text().unwrap();
        let teletext = T::new().parse(HtmlLoader { page_data: body }).unwrap();
        Ok(teletext)
    }

    #[cfg(target_arch = "wasm32")]
    async fn fetch_page(site: &str) -> Result<T, ()> {
        let res = reqwest::Client::new()
            .get(site)
            .send()
            .await
            .map_err(|_| ())?;

        let text = res.text().await.map_err(|_| ())?;
        let teletext = T::new()
            .parse(HtmlLoader { page_data: text })
            .map_err(|_| ())?;
        Ok(teletext)
    }
}

impl HtmlItem {
    pub fn add_to_ui<T: HtmlParser + TelePager + Send + 'static>(
        &self,
        ui: &mut egui::Ui,
        ctx: Rc<RefCell<&mut GuiContext<T>>>,
    ) {
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
    pub fn add_to_ui<T: HtmlParser + TelePager + Send + 'static>(
        &self,
        ui: &mut egui::Ui,
        ctx: Rc<RefCell<&mut GuiContext<T>>>,
    ) {
        if ui.link(&self.inner_text).clicked() {
            ctx.borrow_mut().load_page(&self.url, true);
        }
    }
}
