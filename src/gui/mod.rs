use std::time::Duration;

mod common;
mod yle_text;
use egui::{Color32, FontFamily, FontId, Style, TextStyle, Ui};

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
#[serde(default)] // if we add new fields, give them default values when deserializing old state
struct TeleTextSettings {
    font_size: f32,
    link_color: OptionSetting<[u8; 3]>,
    text_color: OptionSetting<[u8; 3]>,
    background_color: OptionSetting<[u8; 3]>,
    refresh_interval: OptionSetting<u64>,
}

impl Default for TeleTextSettings {
    fn default() -> Self {
        Self {
            font_size: 12.5,
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
    page: Option<GuiYleTextContext>,
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
            egui::FontData::from_static(include_bytes!("../../DejaVuSansMono.ttf")),
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

        let settings = TeleTextSettings::default();
        let mut page = GuiYleTextContext::new(ctx.egui_ctx.clone());
        if settings.refresh_interval.is_used {
            page.set_refresh_interval(settings.refresh_interval.value);
        }

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
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let Self {
            page,
            settings_open,
            settings,
        } = self;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            top_menu_bar(ui, frame, settings_open);
        });

        // .input() locks ctx so we need to copy the data to avoid locks
        let input = ctx.input().to_owned();

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(page) = page {
                page.handle_input(&input);
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

fn top_menu_bar(ui: &mut Ui, frame: &mut eframe::Frame, open: &mut bool) {
    // TODO: hide bar after Settigns etc is clicked
    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui.button("Settings").clicked() {
                *open = true;
            }

            ui.separator();
            if ui.button("Quit").clicked() {
                frame.close();
            }
        });
    });
}

fn settings_window(
    ui: &mut Ui,
    ctx: &egui::Context,
    settings: &mut TeleTextSettings,
    page: &mut GuiYleTextContext,
) {
    if ui
        .add(egui::Slider::new(&mut settings.font_size, 8.0..=24.0).text("Font size"))
        .changed()
    {
        let mut style = (*ctx.style()).clone();
        style.text_styles = [
            (
                TextStyle::Heading,
                FontId::new(settings.font_size, FontFamily::Monospace),
            ),
            (
                TextStyle::Body,
                FontId::new(settings.font_size, FontFamily::Monospace),
            ),
            (
                TextStyle::Monospace,
                FontId::new(settings.font_size, FontFamily::Monospace),
            ),
            (
                TextStyle::Button,
                FontId::new(settings.font_size, FontFamily::Monospace),
            ),
            (
                TextStyle::Small,
                FontId::new(settings.font_size, FontFamily::Monospace),
            ),
        ]
        .into();
        ctx.set_style(style);
    }
    ui.separator();

    egui::Grid::new("settings_grid")
        .num_columns(3)
        .spacing([40.0, 40.0])
        .striped(true)
        .show(ui, |ui| {
            color_option(ui, "Link Color", &mut settings.link_color, |c| {
                let mut visuals = (*ctx.style()).clone().visuals;
                visuals.hyperlink_color = if c.is_used {
                    Color32::from_rgb(c.value[0], c.value[1], c.value[2])
                } else {
                    Style::default().visuals.hyperlink_color
                };
                ctx.set_visuals(visuals);
            });
            color_option(ui, "Text Color", &mut settings.text_color, |c| {
                let mut visuals = (*ctx.style()).clone().visuals;
                visuals.override_text_color = if c.is_used {
                    Some(Color32::from_rgb(c.value[0], c.value[1], c.value[2]))
                } else {
                    None
                };
                ctx.set_visuals(visuals);
            });
            color_option(
                ui,
                "Background Color",
                &mut settings.background_color,
                |c| {
                    let mut visuals = (*ctx.style()).clone().visuals;
                    visuals.panel_fill = if c.is_used {
                        Color32::from_rgb(c.value[0], c.value[1], c.value[2])
                    } else {
                        Style::default().visuals.panel_fill
                    };
                    ctx.set_visuals(visuals);
                },
            );
            ui.label("Refesh interval");
            // FIXME: checkbox changing causes the color edit to not render,
            //        breakign the UI layout for a few frames
            if ui
                .checkbox(&mut settings.refresh_interval.is_used, "use")
                .changed()
            {
                if settings.refresh_interval.is_used {
                    page.set_refresh_interval(settings.refresh_interval.value)
                } else {
                    page.stop_refresh_interval();
                }

                return;
            }

            let interval_val = &mut settings.refresh_interval.value;

            if settings.refresh_interval.is_used
                && ui
                    .add(
                        egui::DragValue::new(interval_val)
                            .speed(1.0)
                            .clamp_range(10..=1800),
                    )
                    .changed()
            {
                page.set_refresh_interval(*interval_val);
            }

            ui.end_row();
        });
}

fn color_option<'a>(
    ui: &mut Ui,
    name: &str,
    color: &'a mut OptionSetting<[u8; 3]>,
    changed: impl FnOnce(&'a mut OptionSetting<[u8; 3]>),
) {
    ui.label(name);
    // FIXME: checkbox changing causes the color edit to not render,
    //        breakign the UI layout for a few frames
    if ui.checkbox(&mut color.is_used, "override").changed() {
        changed(color);
        return;
    }

    if color.is_used && ui.color_edit_button_srgb(&mut color.value).changed() {
        changed(color);
    }

    ui.end_row();
}
