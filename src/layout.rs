pub fn contain_rect(
    viewport_width: f32,
    viewport_height: f32,
    image_width: f32,
    image_height: f32,
) -> eframe::egui::Rect {
    let viewport_size = eframe::egui::vec2(viewport_width.max(1.0), viewport_height.max(1.0));
    let image_size = eframe::egui::vec2(image_width.max(1.0), image_height.max(1.0));
    let scale = (viewport_size.x / image_size.x).min(viewport_size.y / image_size.y);
    let fitted_size = image_size * scale;
    let top_left = eframe::egui::pos2(
        (viewport_size.x - fitted_size.x) * 0.5,
        (viewport_size.y - fitted_size.y) * 0.5,
    );

    eframe::egui::Rect::from_min_size(top_left, fitted_size)
}
