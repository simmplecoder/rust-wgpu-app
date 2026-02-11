pub struct LoadedImage {
    pub width: u32,
    pub height: u32,
    pub rgba8: Vec<u8>,
}

pub fn load_rgba8_from_path(path: &str) -> Result<LoadedImage, String> {
    let dynamic_image =
        image::open(path).map_err(|error| format!("failed to open image at '{path}': {error}"))?;
    let rgba = dynamic_image.to_rgba8();
    let (width, height) = rgba.dimensions();

    Ok(LoadedImage {
        width,
        height,
        rgba8: rgba.into_raw(),
    })
}
