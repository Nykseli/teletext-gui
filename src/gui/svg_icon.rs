use egui::{pos2, Color32, ColorImage, CursorIcon, Mesh, Rect, Sense, Shape, Vec2};
use egui_extras::image::{load_svg_bytes_with_size, FitTo};

#[allow(clippy::enum_variant_names)]
pub enum IconName {
    ArrowUp,
    ArrowRight,
    ArrowDown,
    ArrowLeft,
}

impl IconName {
    fn to_bytes(&self) -> &'static [u8] {
        match self {
            Self::ArrowDown => include_bytes!("../../assets/material_arrow_downward.svg"),
            Self::ArrowUp => include_bytes!("../../assets/material_arrow_upward.svg"),
            Self::ArrowLeft => include_bytes!("../../assets/material_arrow_back.svg"),
            Self::ArrowRight => include_bytes!("../../assets/material_arrow_forward.svg"),
        }
    }

    pub fn to_color_image(&self, color: Color32, size: f32) -> ColorImage {
        let size = size as u32;
        let mut image = load_svg_bytes_with_size(self.to_bytes(), FitTo::Size(size, size))
            .expect("Invalid svg file.");

        for pixel in &mut image.pixels {
            // ignore the transparent pixels
            if pixel[3] == 0 {
                continue;
            }

            pixel[0] = color[0];
            pixel[1] = color[1];
            pixel[2] = color[2];
        }

        image
    }
}

/// Add a underline to `ColorImage` that's under the non-transparent pixels
/// and that's as wide as the image, ignoring non-transparent pixels
fn add_underline(image: &mut ColorImage, color: Color32) {
    let width = image.width();
    let height = image.height();
    // Position of the last pixel on vertical axis, from top to bottom
    let mut last_y = 0;
    let mut x_end = 0;
    let mut x_start = width;

    for (idx, pixel) in image.pixels.iter().enumerate() {
        if pixel[3] == 0 {
            continue;
        }

        let y = idx / width;
        if y > last_y {
            last_y = y;
        }

        let x = idx % width;
        if x > x_end {
            x_end = x;
        }
        if x < x_start {
            x_start = x;
        }
    }

    // Add the underline row one pixel under the last y if there's space
    let row_pos = if last_y == height - 1 {
        last_y
    } else {
        last_y + 1
    };

    let start = row_pos * width + x_start;
    let end = start + x_end - x_start;
    for pixel in &mut image.pixels[start..end] {
        pixel[0] = color[0];
        pixel[1] = color[1];
        pixel[2] = color[2];
    }
}

pub struct SvgIcon {
    // image: ColorImage,
    icon: IconName,
    // color_override: Color32,
    size: f32,
    is_link: bool,
}

impl SvgIcon {
    pub fn from_icon(icon: IconName, size: f32) -> Self {
        // let image = icon.to_color_image(color, size);
        Self {
            // image,
            icon,
            size,
            is_link: false,
        }
    }

    pub fn into_link(mut self) -> Self {
        self.is_link = true;
        self
    }
}

impl egui::Widget for SvgIcon {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(self.size, self.size), Sense::click());

        if !ui.is_rect_visible(rect) {
            return response;
        }

        let color = if self.is_link {
            ui.style().visuals.hyperlink_color
        } else {
            ui.style().visuals.text_color()
        };

        let mut image = self.icon.to_color_image(color, self.size);

        if self.is_link && response.hovered() {
            ui.ctx().output().cursor_icon = CursorIcon::PointingHand;
            add_underline(&mut image, color)
        }

        // TODO: Debug names for all svg textures
        let texture = ui
            .ctx()
            .load_texture("Svg Image", image, Default::default());
        let texture_ref = &texture;

        let mut mesh = Mesh::with_texture(texture_ref.into());
        mesh.add_rect_with_uv(
            rect,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        ui.painter().add(Shape::mesh(mesh));

        response
    }
}
