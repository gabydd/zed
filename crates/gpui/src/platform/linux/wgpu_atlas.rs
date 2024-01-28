use parking_lot::Mutex;

use crate::PlatformAtlas;

struct WgpuAtlasState();
pub(crate) struct WgpuAtlas(Mutex<WgpuAtlasState>);

impl WgpuAtlas {
    pub(crate) fn new() -> Self {
        WgpuAtlas(Mutex::new(WgpuAtlasState()))
    }
}

impl PlatformAtlas for WgpuAtlas {
    fn get_or_insert_with<'a>(
        &self,
        key: &crate::AtlasKey,
        build: &mut dyn FnMut() -> anyhow::Result<(
            crate::Size<crate::DevicePixels>,
            std::borrow::Cow<'a, [u8]>,
        )>,
    ) -> anyhow::Result<crate::AtlasTile> {
        todo!()
    }
}
