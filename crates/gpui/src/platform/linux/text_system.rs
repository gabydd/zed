use collections::HashMap;
use font_kit::{font::Font as FontKitFont, source::SystemSource, sources::mem::MemSource};
use parking_lot::RwLock;
use smallvec::SmallVec;

use crate::{Font, FontId, LineLayout, PlatformTextSystem, Result, SharedString};
use std::{borrow::Cow, sync::Arc};
pub(crate) struct LinuxTextSystem(RwLock<LinuxTextSystemState>);

struct LinuxTextSystemState {
    memory_source: MemSource,
    system_source: SystemSource,
    fonts: Vec<FontKitFont>,
    font_selections: HashMap<Font, FontId>,
    font_ids_by_postscript_name: HashMap<String, FontId>,
    font_ids_by_family_name: HashMap<SharedString, SmallVec<[FontId; 4]>>,
    postscript_names_by_font_id: HashMap<FontId, String>,
}

unsafe impl Send for LinuxTextSystemState {}
unsafe impl Sync for LinuxTextSystemState {}

impl LinuxTextSystemState {}

impl LinuxTextSystem {
    pub(crate) fn new() -> Self {
        Self(RwLock::new(LinuxTextSystemState {
            memory_source: MemSource::empty(),
            system_source: SystemSource::new(),
            fonts: Vec::new(),
            font_selections: HashMap::default(),
            font_ids_by_postscript_name: HashMap::default(),
            font_ids_by_family_name: HashMap::default(),
            postscript_names_by_font_id: HashMap::default(),
        }))
    }
}

impl Default for LinuxTextSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformTextSystem for LinuxTextSystem {
    fn add_fonts(&self, _fonts: Vec<Cow<'static, [u8]>>) -> Result<()> {
        todo!()
    }

    fn all_font_names(&self) -> Vec<String> {
        todo!()
    }

    fn all_font_families(&self) -> Vec<String> {
        todo!()
    }

    fn font_id(&self, _font: &Font) -> Result<FontId> {
        Ok(FontId(0))
    }

    fn font_metrics(&self, _font_id: FontId) -> crate::FontMetrics {
        todo!()
    }

    fn typographic_bounds(
        &self,
        _font_id: FontId,
        _glyph_id: crate::GlyphId,
    ) -> anyhow::Result<crate::Bounds<f32>> {
        todo!()
    }

    fn advance(
        &self,
        _font_id: FontId,
        _glyph_id: crate::GlyphId,
    ) -> anyhow::Result<crate::Size<f32>> {
        todo!()
    }

    fn glyph_for_char(&self, _font_id: FontId, _ch: char) -> Option<crate::GlyphId> {
        todo!()
    }

    fn glyph_raster_bounds(
        &self,
        _params: &crate::RenderGlyphParams,
    ) -> anyhow::Result<crate::Bounds<crate::DevicePixels>> {
        todo!()
    }

    fn rasterize_glyph(
        &self,
        _params: &crate::RenderGlyphParams,
        _raster_bounds: crate::Bounds<crate::DevicePixels>,
    ) -> anyhow::Result<(crate::Size<crate::DevicePixels>, Vec<u8>)> {
        todo!()
    }

    fn layout_line(
        &self,
        _text: &str,
        _font_size: crate::Pixels,
        _runs: &[crate::FontRun],
    ) -> LineLayout {
        LineLayout::default()
    }

    fn wrap_line(
        &self,
        _text: &str,
        _font_id: FontId,
        _font_size: crate::Pixels,
        _width: crate::Pixels,
    ) -> Vec<usize> {
        todo!()
    }
}
