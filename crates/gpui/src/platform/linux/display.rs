use anyhow::anyhow;
use winit::{
    dpi::{LogicalPosition, LogicalSize, Position, Size},
    monitor::MonitorHandle,
    platform::wayland::MonitorHandleExtWayland,
};

use crate::{Bounds, GlobalPixels, PlatformDisplay};

impl From<Bounds<GlobalPixels>> for Size {
    fn from(val: Bounds<GlobalPixels>) -> Self {
        Size::Logical(LogicalSize {
            width: val.size.width.into(),
            height: val.size.height.into(),
        })
    }
}

impl From<Bounds<GlobalPixels>> for Position {
    fn from(val: Bounds<GlobalPixels>) -> Self {
        Position::Logical(LogicalPosition {
            x: val.origin.x.into(),
            y: val.origin.y.into(),
        })
    }
}

#[derive(Debug)]
pub(crate) struct LinuxDisplay(pub(crate) MonitorHandle);
unsafe impl Send for LinuxDisplay {}

impl PlatformDisplay for LinuxDisplay {
    fn id(&self) -> crate::DisplayId {
        crate::DisplayId(self.0.native_id())
    }

    fn uuid(&self) -> anyhow::Result<uuid::Uuid> {
        Err(anyhow!("unimplemented"))
    }

    fn bounds(&self) -> Bounds<GlobalPixels> {
        let size = self.0.size().to_logical(self.0.scale_factor());
        let position = self.0.position().to_logical(self.0.scale_factor());
        Bounds::new(
            crate::Point {
                x: GlobalPixels(position.x),
                y: GlobalPixels(position.y),
            },
            crate::Size {
                width: GlobalPixels(size.width),
                height: GlobalPixels(size.height),
            },
        )
    }
}
