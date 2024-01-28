use std::sync::Arc;

use crate::PlatformWindow;
use crate::{
    Action, BackgroundExecutor, ForegroundExecutor, LinuxDispatcher, LinuxTextSystem, LinuxWindow,
    Platform, PlatformInput, TitlebarOptions, WindowBounds, WindowOptions,
};
use anyhow::anyhow;
use collections::FxHashMap;
use parking_lot::Mutex;
use winit::{
    event_loop::{EventLoopBuilder, EventLoopProxy},
    window::{Window, WindowBuilder, WindowId},
};

pub(crate) struct LinuxPlatform(Arc<Mutex<LinuxPlatformState>>);
pub(crate) struct LinuxPlatformState {
    background_executor: BackgroundExecutor,
    foreground_executor: ForegroundExecutor,
    text_system: Arc<LinuxTextSystem>,
    // Display linker?
    // TODO: clipboard
    become_active: Option<Box<dyn FnMut()>>,
    resign_active: Option<Box<dyn FnMut()>>,
    reopen: Option<Box<dyn FnMut()>>,
    quit: Option<Box<dyn FnMut()>>,
    event: Option<Box<dyn FnMut(PlatformInput) -> bool>>,
    menu_command: Option<Box<dyn FnMut(&dyn Action)>>,
    validate_menu_command: Option<Box<dyn FnMut(&dyn Action) -> bool>>,
    will_open_menu: Option<Box<dyn FnMut()>>,
    menu_actions: Vec<Box<dyn Action>>,
    open_urls: Option<Box<dyn FnMut(Vec<String>)>>,
    finish_launching: Option<Box<dyn FnOnce()>>,
    event_loop: Option<EventLoopProxy<WindowOptions>>,
    channel: Option<smol::channel::Receiver<Window>>,
    windows: FxHashMap<WindowId, LinuxWindow>,
}

unsafe impl Send for LinuxPlatformState {}
unsafe impl Sync for LinuxPlatformState {}
unsafe impl Send for LinuxPlatform {}
unsafe impl Sync for LinuxPlatform {}

impl LinuxPlatform {
    pub(crate) fn new() -> Self {
        let dispatcher = Arc::new(LinuxDispatcher::new());
        Self(Arc::new(Mutex::new(LinuxPlatformState {
            background_executor: BackgroundExecutor::new(dispatcher.clone()),
            foreground_executor: ForegroundExecutor::new(dispatcher),
            text_system: Arc::new(LinuxTextSystem::new()),
            // TODO: clipboard
            become_active: None,
            resign_active: None,
            reopen: None,
            quit: None,
            event: None,
            menu_command: None,
            validate_menu_command: None,
            will_open_menu: None,
            menu_actions: Default::default(),
            open_urls: None,
            finish_launching: None,
            event_loop: None,
            channel: None,
            windows: FxHashMap::default(),
        })))
    }
}

impl Default for LinuxPlatform {
    fn default() -> Self {
        Self::new()
    }
}
impl Platform for LinuxPlatform {
    fn background_executor(&self) -> crate::BackgroundExecutor {
        self.0.lock().background_executor.clone()
    }

    fn foreground_executor(&self) -> crate::ForegroundExecutor {
        self.0.lock().foreground_executor.clone()
    }

    fn text_system(&self) -> std::sync::Arc<dyn crate::PlatformTextSystem> {
        self.0.lock().text_system.clone()
    }

    fn run(&self, on_finish_launching: Box<dyn 'static + FnOnce()>) {
        let event_loop = EventLoopBuilder::<WindowOptions>::with_user_event()
            .build()
            .unwrap();
        self.0.lock().finish_launching = Some(on_finish_launching);

        self.0.lock().event_loop = Some(event_loop.create_proxy().clone());
        let (sender, receiver) = smol::channel::unbounded::<Window>();
        self.0.lock().channel = Some(receiver);
        let state = self.0.clone();
        smol::spawn(async move {
            let this = state.lock().finish_launching.take().unwrap();
            this();
        })
        .detach();
        let _ = event_loop.run(move |event, cx| {
            match event {
                winit::event::Event::UserEvent(options) => {
                    let mut window = WindowBuilder::new();
                    if let Some(TitlebarOptions {
                        title: Some(title), ..
                    }) = options.titlebar
                    {
                        window = window.with_title(title);
                    }

                    match options.bounds {
                        WindowBounds::Fullscreen => {
                            window = window
                                .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
                        }
                        WindowBounds::Maximized => window = window.with_maximized(true),
                        WindowBounds::Fixed(bounds) => {
                            window = window.with_inner_size(bounds).with_position(bounds);
                        }
                    }

                    // TODO: focus it
                    window = window.with_visible(options.show).with_active(options.focus);
                    // TODO: handle popup, center and movable and display
                    let window = window.build(cx).unwrap();
                    smol::block_on(async {
                        let _ = sender.send(window).await;
                    });
                }
                winit::event::Event::NewEvents(_) => (),
                winit::event::Event::WindowEvent { window_id, event } => match event {
                    winit::event::WindowEvent::Resized(new_size) => {
                        let mut windows = self.0.lock().windows.clone();
                        if let Some(window) = windows.get_mut(&window_id) {
                            window.resize(new_size);
                        }
                    }
                    winit::event::WindowEvent::RedrawRequested => {
                        let windows = self.0.lock().windows.clone();
                        if let Some(window) = windows.get(&window_id) {
                            window.redraw();
                        }
                    }
                    winit::event::WindowEvent::CloseRequested => cx.exit(),
                    _ => {}
                },
                winit::event::Event::DeviceEvent {
                    device_id: _,
                    event: _,
                } => (),
                winit::event::Event::Suspended => (),
                winit::event::Event::Resumed => (),
                winit::event::Event::AboutToWait => (),
                winit::event::Event::LoopExiting => (),
                winit::event::Event::MemoryWarning => (),
            }
        });
    }

    fn quit(&self) {
        todo!()
    }

    fn restart(&self) {
        todo!()
    }

    fn activate(&self, _ignoring_other_apps: bool) {
        todo!()
    }

    fn hide(&self) {
        todo!()
    }

    fn hide_other_apps(&self) {
        todo!()
    }

    fn unhide_other_apps(&self) {
        todo!()
    }

    fn displays(&self) -> Vec<std::rc::Rc<dyn crate::PlatformDisplay>> {
        todo!()
    }

    fn display(&self, _id: crate::DisplayId) -> Option<std::rc::Rc<dyn crate::PlatformDisplay>> {
        todo!()
    }

    fn active_window(&self) -> Option<crate::AnyWindowHandle> {
        todo!()
    }

    fn open_window(
        &self,
        handle: crate::AnyWindowHandle,
        options: crate::WindowOptions,
    ) -> Box<dyn crate::PlatformWindow> {
        let _ = self
            .0
            .lock()
            .event_loop
            .clone()
            .unwrap()
            .send_event(options);
        let window = self
            .0
            .lock()
            .channel
            .clone()
            .unwrap()
            .recv_blocking()
            .unwrap();
        let window = Arc::new(window);
        let window = LinuxWindow::open(handle, self.foreground_executor(), window);
        self.0.lock().windows.insert(window.id(), window.clone());
        Box::new(window)
    }

    fn set_display_link_output_callback(
        &self,
        _display_id: crate::DisplayId,
        _callback: Box<dyn FnMut() + Send>,
    ) {
        todo!()
    }

    fn start_display_link(&self, _display_id: crate::DisplayId) {
        todo!()
    }

    fn stop_display_link(&self, _display_id: crate::DisplayId) {
        todo!()
    }

    fn open_url(&self, _url: &str) {
        todo!()
    }

    fn on_open_urls(&self, callback: Box<dyn FnMut(Vec<String>)>) {
        self.0.lock().open_urls = Some(callback);
    }

    fn prompt_for_paths(
        &self,
        _options: crate::PathPromptOptions,
    ) -> futures::channel::oneshot::Receiver<Option<Vec<std::path::PathBuf>>> {
        todo!()
    }

    fn prompt_for_new_path(
        &self,
        _directory: &std::path::Path,
    ) -> futures::channel::oneshot::Receiver<Option<std::path::PathBuf>> {
        todo!()
    }

    fn reveal_path(&self, _path: &std::path::Path) {
        todo!()
    }

    fn on_become_active(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().become_active = Some(callback);
    }

    fn on_resign_active(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().resign_active = Some(callback);
    }

    fn on_quit(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().quit = Some(callback);
    }

    fn on_reopen(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().reopen = Some(callback);
    }

    fn on_event(&self, callback: Box<dyn FnMut(crate::PlatformInput) -> bool>) {
        self.0.lock().event = Some(callback);
    }

    fn set_menus(&self, _menus: Vec<crate::Menu>, _keymapp: &crate::Keymap) {
        todo!()
    }

    fn on_app_menu_action(&self, callback: Box<dyn FnMut(&dyn crate::Action)>) {
        self.0.lock().menu_command = Some(callback);
    }

    fn on_will_open_app_menu(&self, callback: Box<dyn FnMut()>) {
        self.0.lock().will_open_menu = Some(callback);
    }

    fn on_validate_app_menu_command(&self, callback: Box<dyn FnMut(&dyn crate::Action) -> bool>) {
        self.0.lock().validate_menu_command = Some(callback);
    }

    fn os_name(&self) -> &'static str {
        "linux"
    }

    fn os_version(&self) -> anyhow::Result<crate::SemanticVersion> {
        Err(anyhow!("not implemented"))
    }

    fn app_version(&self) -> anyhow::Result<crate::SemanticVersion> {
        Err(anyhow!("not implemented"))
    }

    fn app_path(&self) -> anyhow::Result<std::path::PathBuf> {
        todo!()
    }

    fn local_timezone(&self) -> time::UtcOffset {
        todo!()
    }

    fn double_click_interval(&self) -> std::time::Duration {
        todo!()
    }

    fn path_for_auxiliary_executable(&self, _name: &str) -> anyhow::Result<std::path::PathBuf> {
        todo!()
    }

    fn set_cursor_style(&self, _style: crate::CursorStyle) {
        todo!()
    }

    fn should_auto_hide_scrollbars(&self) -> bool {
        todo!()
    }

    fn write_to_clipboard(&self, _item: crate::ClipboardItem) {
        todo!()
    }

    fn read_from_clipboard(&self) -> Option<crate::ClipboardItem> {
        todo!()
    }

    fn write_credentials(
        &self,
        _url: &str,
        _username: &str,
        _password: &[u8],
    ) -> crate::Task<anyhow::Result<()>> {
        todo!()
    }

    fn read_credentials(
        &self,
        _url: &str,
    ) -> crate::Task<anyhow::Result<Option<(String, Vec<u8>)>>> {
        todo!()
    }

    fn delete_credentials(&self, _url: &str) -> crate::Task<anyhow::Result<()>> {
        todo!()
    }
}
