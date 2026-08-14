use core::fmt;

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext, RenderAssetUsages};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use serde::Deserialize;

use crate::constants::PIXEL_GRID_DIM;

/// The hand-authored JSON schema: a palette of `#RRGGBBAA` hex colors and a
/// flat, row-major array of palette indices for the fixed
/// `PIXEL_GRID_DIM x PIXEL_GRID_DIM` grid. A palette entry with alpha `00`
/// doubles as "empty pixel" — no separate transparency sentinel needed.
#[derive(Deserialize)]
struct AppearanceJson {
    palette: Vec<String>,
    pixels: Vec<u8>,
}

/// A component body's pixel-art appearance, resolved to RGBA8 pixels ready
/// to be baked into a procedural [`Image`] by [`build_image`].
#[derive(Asset, TypePath, Debug)]
pub struct Appearance {
    pixels: Vec<[u8; 4]>,
}

#[derive(Debug)]
pub enum AppearanceError {
    Io(std::io::Error),
    Json(serde_json::Error),
    WrongPixelCount { expected: usize, actual: usize },
    InvalidHexColor(String),
    PaletteIndexOutOfRange { index: u8, palette_len: usize },
}

impl fmt::Display for AppearanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "failed to read appearance file: {err}"),
            Self::Json(err) => write!(f, "failed to parse appearance JSON: {err}"),
            Self::WrongPixelCount { expected, actual } => write!(
                f,
                "appearance has {actual} pixels, expected exactly {expected} ({PIXEL_GRID_DIM}x{PIXEL_GRID_DIM})"
            ),
            Self::InvalidHexColor(hex) => {
                write!(f, "'{hex}' is not a valid #RRGGBBAA hex color")
            }
            Self::PaletteIndexOutOfRange { index, palette_len } => write!(
                f,
                "pixel references palette index {index}, but the palette only has {palette_len} entries"
            ),
        }
    }
}

impl std::error::Error for AppearanceError {}

impl From<std::io::Error> for AppearanceError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<serde_json::Error> for AppearanceError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

fn parse_hex_rgba(hex: &str) -> Result<[u8; 4], AppearanceError> {
    let digits = hex.strip_prefix('#').unwrap_or(hex);
    if digits.len() != 8 {
        return Err(AppearanceError::InvalidHexColor(hex.to_string()));
    }
    let mut bytes = [0u8; 4];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&digits[i * 2..i * 2 + 2], 16)
            .map_err(|_| AppearanceError::InvalidHexColor(hex.to_string()))?;
    }
    Ok(bytes)
}

impl Appearance {
    fn from_json(json: AppearanceJson) -> Result<Self, AppearanceError> {
        let expected = PIXEL_GRID_DIM * PIXEL_GRID_DIM;
        if json.pixels.len() != expected {
            return Err(AppearanceError::WrongPixelCount {
                expected,
                actual: json.pixels.len(),
            });
        }

        let palette = json
            .palette
            .iter()
            .map(|hex| parse_hex_rgba(hex))
            .collect::<Result<Vec<_>, _>>()?;

        let pixels = json
            .pixels
            .iter()
            .map(|&index| {
                palette
                    .get(index as usize)
                    .copied()
                    .ok_or(AppearanceError::PaletteIndexOutOfRange {
                        index,
                        palette_len: palette.len(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { pixels })
    }
}

/// Turns a resolved [`Appearance`] into a procedural, nearest-sampled-ready
/// RGBA8 texture. Relies on [`bevy::image::ImagePlugin::default_nearest`]
/// being configured app-wide for the blocky pixel-art look.
pub fn build_image(appearance: &Appearance) -> Image {
    let mut data = Vec::with_capacity(appearance.pixels.len() * 4);
    for pixel in &appearance.pixels {
        data.extend_from_slice(pixel);
    }
    Image::new(
        Extent3d {
            width: PIXEL_GRID_DIM as u32,
            height: PIXEL_GRID_DIM as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

#[derive(Default, TypePath)]
pub struct AppearanceLoader;

impl AssetLoader for AppearanceLoader {
    type Asset = Appearance;
    type Settings = ();
    type Error = AppearanceError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        // Windows editors (and PowerShell's `-Encoding utf8`) commonly write
        // a UTF-8 BOM, which serde_json rejects outright; strip it so
        // hand-edited appearance files don't need to worry about it.
        let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);
        let json: AppearanceJson = serde_json::from_slice(bytes)?;
        Appearance::from_json(json)
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}

/// Marks a freshly spawned body as still waiting on its appearance JSON to
/// finish loading; the flat-color placeholder `Sprite` set at spawn time
/// stays visible until this resolves.
#[derive(Component)]
pub struct PendingAppearance(pub Handle<Appearance>);

pub fn apply_loaded_appearances(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    appearances: Res<Assets<Appearance>>,
    pending: Query<(Entity, &PendingAppearance, &Sprite)>,
) {
    for (entity, pending_appearance, sprite) in &pending {
        let Some(appearance) = appearances.get(&pending_appearance.0) else {
            continue;
        };
        let handle = images.add(build_image(appearance));
        commands
            .entity(entity)
            .insert(Sprite {
                image: handle,
                custom_size: sprite.custom_size,
                ..default()
            })
            .remove::<PendingAppearance>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_grid_pixels(f: impl Fn(usize) -> u8) -> Vec<u8> {
        (0..PIXEL_GRID_DIM * PIXEL_GRID_DIM).map(f).collect()
    }

    #[test]
    fn from_json_resolves_palette_indices_to_rgba() {
        let json = AppearanceJson {
            palette: vec!["#ff0000ff".to_string(), "#00000000".to_string()],
            pixels: full_grid_pixels(|i| if i % 2 == 0 { 0 } else { 1 }),
        };

        let appearance = Appearance::from_json(json).unwrap();

        assert_eq!(appearance.pixels[0], [0xff, 0x00, 0x00, 0xff]);
        assert_eq!(appearance.pixels[1], [0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn from_json_rejects_wrong_pixel_count() {
        let json = AppearanceJson {
            palette: vec!["#000000ff".to_string()],
            pixels: vec![0; 10],
        };

        let err = Appearance::from_json(json).unwrap_err();
        assert!(matches!(
            err,
            AppearanceError::WrongPixelCount {
                expected: 256,
                actual: 10
            }
        ));
    }

    #[test]
    fn from_json_rejects_out_of_range_palette_index() {
        let mut pixels = full_grid_pixels(|_| 0);
        pixels[42] = 5;
        let json = AppearanceJson {
            palette: vec!["#000000ff".to_string()],
            pixels,
        };

        let err = Appearance::from_json(json).unwrap_err();
        assert!(matches!(
            err,
            AppearanceError::PaletteIndexOutOfRange {
                index: 5,
                palette_len: 1
            }
        ));
    }

    #[test]
    fn from_json_rejects_invalid_hex_color() {
        let json = AppearanceJson {
            palette: vec!["not-a-color".to_string()],
            pixels: full_grid_pixels(|_| 0),
        };

        let err = Appearance::from_json(json).unwrap_err();
        assert!(matches!(err, AppearanceError::InvalidHexColor(_)));
    }

    #[test]
    fn build_image_flattens_pixels_into_rgba8_bytes() {
        let appearance = Appearance {
            pixels: vec![[10, 20, 30, 40]; PIXEL_GRID_DIM * PIXEL_GRID_DIM],
        };

        let image = build_image(&appearance);

        assert_eq!(image.texture_descriptor.size.width, PIXEL_GRID_DIM as u32);
        assert_eq!(image.texture_descriptor.size.height, PIXEL_GRID_DIM as u32);
        let data = image.data.expect("procedurally built image must have data");
        assert_eq!(data.len(), PIXEL_GRID_DIM * PIXEL_GRID_DIM * 4);
        assert_eq!(&data[0..4], &[10, 20, 30, 40]);
    }
}
