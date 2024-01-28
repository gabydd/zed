use std::{rc::Rc, sync::Arc};

use parking_lot::Mutex;
use winit::{
    dpi::PhysicalSize,
    window::{Window, WindowId},
};

use crate::{
    AnyWindowHandle, Bounds, ForegroundExecutor, GlobalPixels, LinuxDisplay, Pixels, PlatformInput,
    PlatformWindow, Size, WgpuAtlas, WgpuRenderer, WindowBounds,
};

struct LinuxWindowState {
    renderer: WgpuRenderer,
    window: Arc<Window>,
    request_frame_callback: Option<Box<dyn FnMut()>>,
    event_callback: Option<Box<dyn FnMut(PlatformInput) -> bool>>,
    activate_callback: Option<Box<dyn FnMut(bool)>>,
    resize_callback: Option<Box<dyn FnMut(Size<Pixels>, f32)>>,
    fullscreen_callback: Option<Box<dyn FnMut(bool)>>,
    moved_callback: Option<Box<dyn FnMut()>>,
    should_close_callback: Option<Box<dyn FnMut() -> bool>>,
    close_callback: Option<Box<dyn FnOnce()>>,
    appearance_changed_callback: Option<Box<dyn FnMut()>>,
}

unsafe impl Send for LinuxWindowState {}
unsafe impl Sync for LinuxWindowState {}

pub(crate) struct LinuxWindow(Arc<Mutex<LinuxWindowState>>);
impl Clone for LinuxWindow {
    fn clone(&self) -> Self {
        LinuxWindow(self.0.clone())
    }
}
impl LinuxWindow {
    pub fn open(
        _handle: AnyWindowHandle,
        _executor: ForegroundExecutor,
        window: Arc<Window>,
    ) -> Self {
        Self(Arc::new(Mutex::new(LinuxWindowState {
            renderer: WgpuRenderer::new(window.clone()),
            window,
            request_frame_callback: None,
            event_callback: None,
            activate_callback: None,
            resize_callback: None,
            fullscreen_callback: None,
            moved_callback: None,
            should_close_callback: None,
            close_callback: None,
            appearance_changed_callback: None,
        })))
    }
    pub(crate) fn id(&self) -> WindowId {
        self.0.lock().window.id()
    }

    pub(crate) fn resize(&mut self, new_size: PhysicalSize<u32>) {
        let mut this = self.0.lock();
        this.renderer.resize(new_size);
        this.window.request_redraw();
    }

    pub(crate) fn redraw(&self) {
        let mut this = self.0.lock();
        if let Some(mut callback) = this.request_frame_callback.take() {
            drop(this);
            callback();
            self.0.lock().request_frame_callback = Some(callback);
        }
    }
}
impl PlatformWindow for LinuxWindow {
    fn bounds(&self) -> crate::WindowBounds {
        let window = &self.0.lock().window;
        if window.is_maximized() {
            return WindowBounds::Maximized;
        }
        match window.fullscreen() {
            None => (),
            Some(_) => return WindowBounds::Fullscreen,
        }
        let size = window.inner_size().to_logical(window.scale_factor());
        if let Ok(position) = window.inner_position() {
            let position = position.to_logical(window.scale_factor());
            WindowBounds::Fixed(Bounds::new(
                crate::Point {
                    x: GlobalPixels(position.x),
                    y: GlobalPixels(position.y),
                },
                crate::Size {
                    width: GlobalPixels(size.width),
                    height: GlobalPixels(size.height),
                },
            ))
        } else {
            WindowBounds::Maximized
        }
    }

    fn content_size(&self) -> crate::Size<crate::Pixels> {
        let this = self.0.lock();
        let size = this
            .window
            .inner_size()
            .to_logical(this.window.scale_factor());
        crate::Size {
            width: crate::Pixels(size.width),
            height: crate::Pixels(size.height),
        }
    }

    fn scale_factor(&self) -> f32 {
        self.0.lock().window.scale_factor() as f32
    }

    fn titlebar_height(&self) -> crate::Pixels {
        todo!()
    }

    fn appearance(&self) -> crate::WindowAppearance {
        todo!()
    }

    fn display(&self) -> std::rc::Rc<dyn crate::PlatformDisplay> {
        Rc::new(LinuxDisplay(
            self.0.lock().window.available_monitors().next().unwrap(),
        ))
    }

    fn mouse_position(&self) -> crate::Point<crate::Pixels> {
        crate::Point::default()
    }

    fn modifiers(&self) -> crate::Modifiers {
        crate::Modifiers::default()
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        todo!()
    }

    fn set_input_handler(&mut self, _input_handler: crate::PlatformInputHandler) {
        todo!()
    }

    fn take_input_handler(&mut self) -> Option<crate::PlatformInputHandler> {
        todo!()
    }

    fn prompt(
        &self,
        _level: crate::PromptLevel,
        _msg: &str,
        _detail: Option<&str>,
        _answers: &[&str],
    ) -> futures::channel::oneshot::Receiver<usize> {
        todo!()
    }

    fn activate(&self) {
        todo!()
    }

    fn set_title(&mut self, _title: &str) {
        todo!()
    }

    fn set_edited(&mut self, _edited: bool) {
        todo!()
    }

    fn show_character_palette(&self) {
        todo!()
    }

    fn minimize(&self) {
        todo!()
    }

    fn zoom(&self) {
        todo!()
    }

    fn toggle_full_screen(&self) {
        todo!()
    }

    fn on_request_frame(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().request_frame_callback = Some(callback);
    }

    fn on_input(&self, callback: Box<dyn FnMut(crate::PlatformInput) -> bool>) {
        self.0.lock().event_callback = Some(callback);
    }

    fn on_active_status_change(&self, callback: Box<dyn FnMut(bool)>) {
        self.0.lock().activate_callback = Some(callback);
    }

    fn on_resize(&self, callback: Box<dyn FnMut(crate::Size<crate::Pixels>, f32)>) {
        self.0.as_ref().lock().resize_callback = Some(callback);
    }

    fn on_fullscreen(&self, callback: Box<dyn FnMut(bool)>) {
        self.0.as_ref().lock().fullscreen_callback = Some(callback);
    }

    fn on_moved(&self, callback: Box<dyn FnMut()>) {
        self.0.as_ref().lock().moved_callback = Some(callback);
    }

    fn on_should_close(&self, callback: Box<dyn FnMut() -> bool>) {
        self.0.as_ref().lock().should_close_callback = Some(callback);
    }

    fn on_close(&self, callback: Box<dyn FnOnce()>) {
        self.0.as_ref().lock().close_callback = Some(callback);
    }

    fn on_appearance_changed(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().appearance_changed_callback = Some(callback);
    }

    fn is_topmost_for_position(&self, _position: crate::Point<crate::Pixels>) -> bool {
        todo!()
    }

    fn invalidate(&self) {
        self.0.lock().window.request_redraw();
    }

    fn draw(&self, scene: &crate::Scene) {
        let this = self.0.lock();
        this.renderer.draw(scene);
    }

    fn sprite_atlas(&self) -> std::sync::Arc<dyn crate::PlatformAtlas> {
        Arc::new(WgpuAtlas::new())
    }
}
