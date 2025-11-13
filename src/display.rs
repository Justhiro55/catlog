use anyhow::Result;
use image::io::Reader as ImageReader;
use std::io::Cursor;
use viuer::Config;

pub fn display_image(image_data: &[u8], size: u32, _ascii: bool) -> Result<()> {
    // Load image from bytes
    let img = ImageReader::new(Cursor::new(image_data))
        .with_guessed_format()?
        .decode()?;

    // Configure viuer
    let conf = Config {
        transparent: true,
        absolute_offset: false,
        width: Some(size),
        height: None,
        ..Default::default()
    };

    // Display the image
    viuer::print(&img, &conf)?;

    Ok(())
}
