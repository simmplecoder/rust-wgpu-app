use eframe::{egui_wgpu::RenderState, CreationContext};

use crate::renderer::ComputeRenderer;
use crate::image_io::{self, LoadedImage};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct InternalState {
    image_path: PathBuf,
    output_texture: eframe::egui::TextureHandle,
    image_size: eframe::egui::Vec2,
}

pub struct CvApp {
    render_state: Option<Arc<RenderState>>,
    compute_renderer: Option<ComputeRenderer>,
    last_error: Option<String>,
    state: Option<InternalState>,
}

impl CvApp {
    pub fn new(creation_context: &CreationContext<'_>) -> Self {
        CvApp {
            render_state: creation_context
                .wgpu_render_state
                .clone()
                .map(Arc::new),
            compute_renderer: None,
            last_error: None,
            state: None,
        }
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

        if self.compute_renderer.is_none() {
            self.compute_renderer = Some(
                ComputeRenderer::new(&render_state.device).map_err(|error| error.to_string())?,
            );
        }

        let output_image = self
            .compute_renderer
            .as_ref()
            .expect("renderer initialized above")
            .process_image(&render_state.device, &render_state.queue, &source_image)
            .map_err(|error| error.to_string())?;
        let output_image = CvApp::downscale_if_needed(output_image, 2048)?;
        let color_image = eframe::egui::ColorImage::from_rgba_unmultiplied(
            [output_image.width as usize, output_image.height as usize],
            &output_image.rgba8,
        );
        let output_texture = ctx.load_texture(
            "remove_red_output",
            color_image,
            eframe::egui::TextureOptions::LINEAR,
        );
        self.state = Some(InternalState {
            image_path: pathbuf,
            output_texture,
            image_size: eframe::egui::Vec2::new(output_image.width as f32, output_image.height as f32),
        });

        Ok(())
    }
}

impl eframe::App for CvApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        eframe::egui::CentralPanel::default()
            .frame(eframe::egui::Frame::new().fill(eframe::egui::Color32::BLACK))
            .show(ctx, |ui| {
                let choose_image_button = ui.button("Choose image");
                if choose_image_button.clicked() {
                    let picked = rfd::FileDialog::new()
                        .add_filter(
                            "Image",
                            &[
                                "png", "jpg", "jpeg", "bmp", "gif", "tif", "tiff", "webp", "ico",
                                "avif",
                            ],
                        )
                        .pick_file();

                    if let Some(path) = picked {
                        let Some(render_state) = self.render_state.clone() else {
                            self.last_error = Some(
                                "wgpu render state unavailable: ensure eframe uses wgpu backend"
                                    .to_owned(),
                            );
                            return;
                        };

                        match self.process_image_path(ctx, render_state.as_ref(), &path) {
                            Ok(()) => {
                                self.last_error = None;
                                ctx.request_repaint();
                            }
                            Err(e) => {
                                self.last_error = Some(e);
                            }
                        }
                    }
                }

                if let Some(error) = &self.last_error {
                    ui.colored_label(eframe::egui::Color32::LIGHT_RED, error);
                }

                let viewport_size = ui.available_size_before_wrap();
                let (_id, viewport_rect) = ui.allocate_space(viewport_size);
                let Some(state) = &self.state else {
                    return;
                };

                let output_texture = &state.output_texture;
                let fitted_local = crate::layout::contain_rect(
                    viewport_rect.width(),
                    viewport_rect.height(),
                    state.image_size.x,
                    state.image_size.y,
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
            });
    }
}
