use eframe::{egui_wgpu::RenderState, CreationContext};

use crate::gpu::compute_remove_red;
use crate::image_io::{self, LoadedImage};
use std::path::{Path, PathBuf};

pub struct CvApp {
    current_image_path: Option<PathBuf>,
    output_texture: Option<eframe::egui::TextureHandle>,
    image_size: eframe::egui::Vec2,
    startup_error: Option<String>,
}

impl CvApp {
    pub fn new(creation_context: &eframe::CreationContext<'_>, image_path: &str) -> Self {
        match Self::initialize(creation_context, image_path) {
            Ok((output_texture, image_size)) => Self {
                current_image_path: None,
                output_texture: Some(output_texture),
                image_size,
                startup_error: None,
            },
            Err(error) => Self {
                current_image_path: None,
                output_texture: None,
                image_size: eframe::egui::vec2(1.0, 1.0),
                startup_error: Some(error),
            },
        }
    }

    fn initialize(
        creation_context: &eframe::CreationContext<'_>,
        image_path: &str,
    ) -> Result<(eframe::egui::TextureHandle, eframe::egui::Vec2), String> {

    }

    fn downscale_if_needed(img: LoadedImage, max_side: u32) -> Result<LoadedImage, String> {
        if img.width <= max_side && img.height <= max_side {
            return Ok(img);
        }

        let scale = (max_side as f32 / img.width as f32).min(max_side as f32 / img.height as f32);

        let new_w = ((img.width as f32 * scale).round() as u32).max(1);
        let new_h = ((img.height as f32 * scale).round() as u32).max(1);

        let src = image::RgbaImage::from_raw(img.width, img.height, img.rgba8)
            .ok_or_else(|| "invalid RGBA buffer size".to_owned())?;

        let resized =
            image::imageops::resize(&src, new_w, new_h, image::imageops::FilterType::Triangle);

        Ok(crate::image_io::LoadedImage {
            width: new_w,
            height: new_h,
            rgba8: resized.into_raw(),
        })
    }

    fn process_image_path(
        &mut self,
        ctx: &eframe::egui::Context,
        render_state: &RenderState,
        path: &Path,
    ) -> Result<(), String> {
        let pathbuf = path.to_path_buf();
        let Some(path) = path.as_os_str().to_str() else {
            return Err(String::from("Selected file path is not valid UTF-8"));
        };
        let source_image = image_io::load_rgba8_from_path(path)?;
        self.current_image_path = Some(pathbuf);
        

        return Ok(());
    }
}

impl eframe::App for CvApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        eframe::egui::CentralPanel::default()
            .frame(eframe::egui::Frame::new().fill(eframe::egui::Color32::BLACK))
            .show(ctx, |ui| {
                let viewport_size = ui.available_size_before_wrap();
                let (_id, viewport_rect) = ui.allocate_space(viewport_size);

                if let Some(output_texture) = &self.output_texture {
                    let fitted_local = crate::layout::contain_rect(
                        viewport_rect.width(),
                        viewport_rect.height(),
                        self.image_size.x,
                        self.image_size.y,
                    );
                    let fitted_rect = fitted_local.translate(viewport_rect.min.to_vec2());
                    let uv = eframe::egui::Rect::from_min_max(
                        eframe::egui::pos2(0.0, 0.0),
                        eframe::egui::pos2(1.0, 1.0),
                    );
                    ui.painter().image(
                        output_texture.id(),
                        fitted_rect,
                        uv,
                        eframe::egui::Color32::WHITE,
                    );
                } else if let Some(error) = &self.startup_error {
                    let font = eframe::egui::TextStyle::Heading.resolve(ui.style());
                    ui.painter().text(
                        viewport_rect.center(),
                        eframe::egui::Align2::CENTER_CENTER,
                        error,
                        font,
                        eframe::egui::Color32::LIGHT_RED,
                    );
                }
            });
    }
}
