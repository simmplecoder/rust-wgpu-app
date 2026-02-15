mod app;
mod renderer;
mod image_io;
mod layout;

const HARDCODED_IMAGE_PATH: &str =
    r"C:\Users\anony\Pictures\Screenshots\Screenshot 2024-10-13 165336.png";

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Image Processing App")
            .with_inner_size([1920.0, 1080.0])
            .with_min_inner_size([1920.0, 1080.0])
            .with_max_inner_size([1920.0, 1080.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "Image Processing App",
        native_options,
        Box::new(|creation_context| {
            Ok(Box::new(app::CvApp::new(creation_context)))
        }),
    )
}
