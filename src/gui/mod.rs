use std::time::Duration;

mod common;
mod yle_image;
mod yle_text;
use egui::{Color32, FontFamily, FontId, Style, TextStyle, Ui};

use self::common::{GuiContext, IGuiCtx};
use self::yle_image::GuiYleImageContext;
use self::yle_text::GuiYleTextContext;

#[derive(Default, serde::Deserialize, serde::Serialize)]
struct OptionSetting<T> {
    is_used: bool,
    value: T,
}

fn def_color_opt(color: [u8; 3]) -> OptionSetting<[u8; 3]> {
    OptionSetting {
        is_used: false,
        value: color,
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
enum Pages {
    YleText,
    YleImage,
}

impl Pages {
    fn to_gui(&self, egui: &egui::Context) -> Box<dyn IGuiCtx> {
        match self {
            Self::YleImage => {
                Box::new(GuiYleImageContext::new(GuiContext::new(egui.clone()))) as Box<dyn IGuiCtx>
            }
            Self::YleText => {
                Box::new(GuiYleTextContext::new(GuiContext::new(egui.clone()))) as Box<dyn IGuiCtx>
            }
        }
    }
}

impl Default for Pages {
    fn default() -> Self {
        Self::YleText
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
struct TeleTextSettings {
    font_size: f32,
    open_page: Pages,
    link_color: OptionSetting<[u8; 3]>,
    text_color: OptionSetting<[u8; 3]>,
    background_color: OptionSetting<[u8; 3]>,
    refresh_interval: OptionSetting<u64>,
}

impl TeleTextSettings {
    /// Initialize all settings, should be used when app is initalised
    fn init_all(&self, ctx: &egui::Context, page: &mut Box<dyn IGuiCtx>) {
        self.set_colors(ctx);
        self.set_font_size(ctx);
        self.set_refresh_interval(page);
    }

    fn set_colors(&self, ctx: &egui::Context) {
        let mut visuals = (*ctx.style()).clone().visuals;
        let defaults = Style::default().visuals;

        let c = &self.link_color;
        visuals.hyperlink_color = if c.is_used {
            Color32::from_rgb(c.value[0], c.value[1], c.value[2])
        } else {
            defaults.hyperlink_color
        };

        let c = &self.text_color;
        visuals.override_text_color = if c.is_used {
            Some(Color32::from_rgb(c.value[0], c.value[1], c.value[2]))
        } else {
            defaults.override_text_color
        };

        let c = &self.background_color;
        visuals.panel_fill = if c.is_used {
            Color32::from_rgb(c.value[0], c.value[1], c.value[2])
        } else {
            defaults.panel_fill
        };

        ctx.set_visuals(visuals);
    }

    fn set_font_size(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        style.text_styles = [
            (
                TextStyle::Heading,
                FontId::new(self.font_size, FontFamily::Monospace),
            ),
            (
                TextStyle::Body,
                FontId::new(self.font_size, FontFamily::Monospace),
            ),
            (
                TextStyle::Monospace,
                FontId::new(self.font_size, FontFamily::Monospace),
            ),
            (
                TextStyle::Button,
                FontId::new(self.font_size, FontFamily::Monospace),
            ),
            (
                TextStyle::Small,
                FontId::new(self.font_size, FontFamily::Monospace),
            ),
        ]
        .into();
        ctx.set_style(style);
    }

    fn set_refresh_interval(&self, page: &mut Box<dyn IGuiCtx>) {
        if self.refresh_interval.is_used {
            page.set_refresh_interval(self.refresh_interval.value);
        } else {
            page.stop_refresh_interval();
        }
    }
}

impl Default for TeleTextSettings {
    fn default() -> Self {
        Self {
            font_size: 12.5,
            open_page: Default::default(),
            link_color: def_color_opt([17, 159, 244]),
            text_color: def_color_opt([255, 255, 255]),
            background_color: def_color_opt([0, 0, 0]),
            refresh_interval: OptionSetting {
                is_used: false,
                value: 300,
            },
        }
    }
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TeleTextApp {
    #[serde(skip)]
    page: Option<Box<dyn IGuiCtx>>,
    #[serde(skip)]
    settings_open: bool,
    settings: TeleTextSettings,
}

impl TeleTextApp {
    /// Called once before the first frame.
    pub fn new(ctx: &eframe::CreationContext<'_>) -> Self {
        // Override default fonts with our own font
        let mut fonts = egui::FontDefinitions::empty();
        fonts.font_data.insert(
            "default_font".to_owned(),
            egui::FontData::from_static(include_bytes!("../../assets/DejaVuSansMono.ttf")),
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

        let settings = if let Some(storage) = ctx.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            TeleTextSettings::default()
        };

        let mut page = settings.open_page.to_gui(&ctx.egui_ctx);
        let page_ref = &mut page as &mut Box<dyn IGuiCtx>;

        settings.init_all(&ctx.egui_ctx, page_ref);

        Self {
            page: Some(page),
            settings_open: false,
            settings,
        }
    }
}

impl eframe::App for TeleTextApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.settings);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let Self {
            page,
            settings_open,
            settings,
        } = self;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            top_menu_bar(ui, ctx, frame, settings_open, page, settings);
        });

        // .input() locks ctx so we need to copy the data to avoid locks
        let input = ctx.input().to_owned();

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(page) = page {
                page.handle_input(input);
                page.draw(ui);
            }
        });

        egui::Window::new("Settings")
            .open(settings_open)
            .show(ctx, |ui| {
                if let Some(page) = page {
                    settings_window(ui, ctx, settings, page);
                }
            });

        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

fn top_menu_bar(
    ui: &mut Ui,
    egui: &egui::Context,
    frame: &mut eframe::Frame,
    open: &mut bool,
    page: &mut Option<Box<dyn IGuiCtx>>,
    settings: &mut TeleTextSettings,
) {
    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            ui.menu_button("Reader", |ui| {
                if ui.button("Yle Text").clicked() {
                    settings.open_page = Pages::YleText;
                    *page = Some(Pages::YleText.to_gui(egui));
                    ui.close_menu();
                }

                if ui.button("Yle Image").clicked() {
                    settings.open_page = Pages::YleImage;
                    *page = Some(Pages::YleImage.to_gui(egui));
                    ui.close_menu();
                }
            });

            if ui.button("Settings").clicked() {
                *open = true;
                ui.close_menu();
            }

            ui.separator();
            if ui.button("Quit").clicked() {
                frame.close();
                ui.close_menu();
            }
        });
    });
}

fn settings_window(
    ui: &mut Ui,
    ctx: &egui::Context,
    settings: &mut TeleTextSettings,
    page: &mut Box<dyn IGuiCtx>,
) {
    if ui
        .add(egui::Slider::new(&mut settings.font_size, 8.0..=48.0).text("Font size"))
        .changed()
    {
        settings.set_font_size(ctx);
    }
    ui.separator();

    egui::Grid::new("settings_grid")
        .num_columns(3)
        .spacing([40.0, 40.0])
        .striped(true)
        .show(ui, |ui| {
            if color_option(ui, "Link Color", &mut settings.link_color) {
                settings.set_colors(ctx);
            }

            if color_option(ui, "Text Color", &mut settings.text_color) {
                settings.set_colors(ctx);
            }

            if color_option(ui, "Background Color", &mut settings.background_color) {
                settings.set_colors(ctx);
            }

            ui.label("Refesh interval");
            if ui
                .checkbox(&mut settings.refresh_interval.is_used, "use")
                .changed()
            {
                settings.set_refresh_interval(page);
            }

            let interval_val = &mut settings.refresh_interval.value;

            if settings.refresh_interval.is_used
                && ui
                    .add(
                        egui::DragValue::new(interval_val)
                            .speed(1.0)
                            .clamp_range(30..=1800),
                    )
                    .changed()
            {
                settings.set_refresh_interval(page);
            }

            ui.end_row();
        });
}

fn color_option(ui: &mut Ui, name: &str, color: &mut OptionSetting<[u8; 3]>) -> bool {
    let mut changed = false;
    ui.label(name);
    if ui.checkbox(&mut color.is_used, "override").changed() {
        changed = true
    }

    if color.is_used && ui.color_edit_button_srgb(&mut color.value).changed() {
        changed = true;
    }

    ui.end_row();
    changed
}
