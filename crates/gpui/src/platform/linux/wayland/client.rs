use parking_lot::Mutex;
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::seat::keyboard::{KeyboardHandler, Keysym};
use smithay_client_toolkit::seat::pointer::{PointerEventKind, PointerHandler};
use smithay_client_toolkit::seat::{Capability, SeatHandler, SeatState};
use smithay_client_toolkit::{
    delegate_keyboard, delegate_pointer, delegate_registry, delegate_seat, registry_handlers,
};
use std::rc::Rc;
use std::sync::Arc;
use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::wl_callback::WlCallback;
use wayland_client::protocol::wl_keyboard::WlKeyboard;
use wayland_client::protocol::wl_pointer::WlPointer;
use wayland_client::{
    delegate_noop,
    protocol::{
        wl_buffer, wl_callback, wl_compositor, wl_keyboard, wl_registry, wl_seat, wl_shm,
        wl_shm_pool,
        wl_surface::{self, WlSurface},
    },
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
};

use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

use crate::platform::linux::client::Client;
use crate::platform::linux::wayland::window::WaylandWindow;
use crate::platform::{LinuxPlatformInner, PlatformWindow};
use crate::{
    platform::linux::wayland::window::WaylandWindowState, AnyWindowHandle, DisplayId,
    PlatformDisplay, WindowOptions,
};
use crate::{point, KeyDownEvent, Modifiers, MouseButton, PlatformInput, ScrollDelta, TouchPhase};

pub(crate) struct WaylandClientState {
    compositor: Option<wl_compositor::WlCompositor>,
    buffer: Option<wl_buffer::WlBuffer>,
    wm_base: Option<xdg_wm_base::XdgWmBase>,
    windows: Vec<(xdg_surface::XdgSurface, Arc<WaylandWindowState>)>,
    registry_state: RegistryState,
    seat_state: SeatState,
    keyboard: Option<WlKeyboard>,
    pointer: Option<WlPointer>,
    window: Option<Arc<WaylandWindowState>>,
    modifiers: Modifiers,
    scrolling: bool,
    pressed_button: Option<MouseButton>,
}

pub(crate) struct WaylandClient {
    platform_inner: Arc<LinuxPlatformInner>,
    conn: Arc<Connection>,
    state: Mutex<WaylandClientState>,
    event_queue: Mutex<EventQueue<WaylandClientState>>,
    qh: Arc<QueueHandle<WaylandClientState>>,
}

impl WaylandClient {
    pub(crate) fn new(
        linux_platform_inner: Arc<LinuxPlatformInner>,
        conn: Arc<Connection>,
    ) -> Self {
        let (global_list, event_queue) = registry_queue_init(&conn).unwrap();
        let state = WaylandClientState {
            pressed_button: None,
            scrolling: false,
            compositor: None,
            buffer: None,
            wm_base: None,
            windows: Vec::new(),
            registry_state: RegistryState::new(&global_list),
            seat_state: SeatState::new(&global_list, &event_queue.handle()),
            keyboard: None,
            pointer: None,
            window: None,
            modifiers: Modifiers::default(),
        };
        let qh = event_queue.handle();
        Self {
            platform_inner: linux_platform_inner,
            conn,
            state: Mutex::new(state),
            event_queue: Mutex::new(event_queue),
            qh: Arc::new(qh),
        }
    }
}

impl Client for WaylandClient {
    fn run(&self, on_finish_launching: Box<dyn FnOnce()>) {
        let display = self.conn.display();
        let mut eq = self.event_queue.lock();
        let _registry = display.get_registry(&self.qh, ());

        eq.roundtrip(&mut self.state.lock()).unwrap();

        on_finish_launching();
        while !self.platform_inner.state.lock().quit_requested {
            eq.flush().unwrap();
            eq.dispatch_pending(&mut self.state.lock()).unwrap();
            if let Some(guard) = self.conn.prepare_read() {
                guard.read().unwrap();
                eq.dispatch_pending(&mut self.state.lock()).unwrap();
            }
            if let Ok(runnable) = self.platform_inner.main_receiver.try_recv() {
                runnable.run();
            }
        }
    }

    fn displays(&self) -> Vec<Rc<dyn PlatformDisplay>> {
        Vec::new()
    }

    fn display(&self, id: DisplayId) -> Option<Rc<dyn PlatformDisplay>> {
        todo!()
    }

    fn open_window(
        &self,
        handle: AnyWindowHandle,
        options: WindowOptions,
    ) -> Box<dyn PlatformWindow> {
        let mut state = self.state.lock();

        let wm_base = state.wm_base.as_ref().unwrap();
        let compositor = state.compositor.as_ref().unwrap();
        let wl_surface = compositor.create_surface(&self.qh, ());
        let xdg_surface = wm_base.get_xdg_surface(&wl_surface, &self.qh, ());
        let toplevel = xdg_surface.get_toplevel(&self.qh, ());
        let wl_surface = Arc::new(wl_surface);

        wl_surface.frame(&self.qh, wl_surface.clone());
        wl_surface.commit();

        let window_state: Arc<WaylandWindowState> = Arc::new(WaylandWindowState::new(
            &self.conn,
            wl_surface.clone(),
            Arc::new(toplevel),
            options,
        ));
        // window_state.update();

        state.windows.push((xdg_surface, Arc::clone(&window_state)));
        Box::new(WaylandWindow(window_state))
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for WaylandClientState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name, interface, ..
        } = event
        {
            match &interface[..] {
                "wl_compositor" => {
                    let compositor =
                        registry.bind::<wl_compositor::WlCompositor, _, _>(name, 1, qh, ());
                    state.compositor = Some(compositor);
                }
                "xdg_wm_base" => {
                    let wm_base = registry.bind::<xdg_wm_base::XdgWmBase, _, _>(name, 1, qh, ());
                    state.wm_base = Some(wm_base);
                }
                _ => {}
            };
        }
    }
}

delegate_noop!(WaylandClientState: ignore wl_compositor::WlCompositor);
delegate_noop!(WaylandClientState: ignore wl_surface::WlSurface);
delegate_noop!(WaylandClientState: ignore wl_shm::WlShm);
delegate_noop!(WaylandClientState: ignore wl_shm_pool::WlShmPool);
delegate_noop!(WaylandClientState: ignore wl_buffer::WlBuffer);
delegate_noop!(WaylandClientState: ignore wl_seat::WlSeat);
delegate_noop!(WaylandClientState: ignore wl_keyboard::WlKeyboard);

impl Dispatch<WlCallback, Arc<WlSurface>> for WaylandClientState {
    fn event(
        state: &mut Self,
        _: &WlCallback,
        event: wl_callback::Event,
        surf: &Arc<WlSurface>,
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = event {
            for window in &state.windows {
                if window.1.surface.id() == surf.id() {
                    window.1.surface.frame(qh, surf.clone());
                    window.1.update();
                    window.1.surface.commit();
                }
            }
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for WaylandClientState {
    fn event(
        state: &mut Self,
        xdg_surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial, .. } = event {
            xdg_surface.ack_configure(serial);
            for window in &state.windows {
                if &window.0 == xdg_surface {
                    window.1.update();
                    window.1.surface.commit();
                    return;
                }
            }
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for WaylandClientState {
    fn event(
        state: &mut Self,
        xdg_toplevel: &xdg_toplevel::XdgToplevel,
        event: <xdg_toplevel::XdgToplevel as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_toplevel::Event::Configure {
            width,
            height,
            states,
        } = event
        {
            if width == 0 || height == 0 {
                return;
            }
            for window in &state.windows {
                if window.1.toplevel.id() == xdg_toplevel.id() {
                    window.1.resize(width, height);
                    window.1.surface.commit();
                    return;
                }
            }
        }
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for WaylandClientState {
    fn event(
        state: &mut Self,
        wm_base: &xdg_wm_base::XdgWmBase,
        event: <xdg_wm_base::XdgWmBase as wayland_client::Proxy>::Event,
        data: &(),
        conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm_base.pong(serial);
        }
    }
}

impl KeyboardHandler for WaylandClientState {
    fn enter(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        serial: u32,
        raw: &[u32],
        keysyms: &[smithay_client_toolkit::seat::keyboard::Keysym],
    ) {
        for window in &self.windows {
            if window.1.surface.id() == surface.id() {
                self.window = Some(window.1.clone());
                return;
            }
        }
    }

    fn leave(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        serial: u32,
    ) {
        self.window = None;
    }

    fn press_key(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &wl_keyboard::WlKeyboard,
        serial: u32,
        event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
        if let Some(window) = self.window.clone() {
            if let Some(key) = keysym_to_key(event.keysym).or(event.utf8) {
                window.handle_key(
                    KeyDownEvent {
                        keystroke: crate::Keystroke {
                            modifiers: self.modifiers,
                            key: key.clone().to_lowercase(),
                            ime_key: None,
                        },
                        is_held: false,
                    },
                    &key,
                );
            }
        }
    }

    fn release_key(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &wl_keyboard::WlKeyboard,
        serial: u32,
        event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
        if let Some(window) = self.window.clone() {
            window.handle_event(PlatformInput::KeyUp(crate::KeyUpEvent {
                keystroke: crate::Keystroke {
                    modifiers: self.modifiers,
                    key: "".to_string(),
                    ime_key: None,
                },
            }))
        }
    }

    fn update_modifiers(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        keyboard: &wl_keyboard::WlKeyboard,
        serial: u32,
        modifiers: smithay_client_toolkit::seat::keyboard::Modifiers,
    ) {
        self.modifiers = Modifiers {
            control: modifiers.ctrl,
            alt: modifiers.alt,
            shift: modifiers.shift,
            command: modifiers.logo,
            function: false,
        }
    }
}

fn keysym_to_key(keysym: Keysym) -> Option<String> {
    Some(
        match keysym {
            Keysym::BackSpace => "backspace",
            Keysym::Down => "down",
            Keysym::Up => "up",
            Keysym::Left => "left",
            Keysym::Right => "right",
            Keysym::Delete => "delete",
            Keysym::Page_Up => "pageup",
            Keysym::Page_Down => "pagedown",
            Keysym::Home => "home",
            Keysym::End => "end",
            Keysym::Escape => "escape",
            Keysym::Return => "enter",
            Keysym::space => "space",
            Keysym::Tab => "tab",
            _ => return None,
        }
        .to_string(),
    )
}
impl SeatHandler for WaylandClientState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            println!("Set keyboard capability");
            let keyboard = self
                .seat_state
                .get_keyboard(qh, &seat, None)
                .expect("Failed to create keyboard");

            self.keyboard = Some(keyboard);
        }

        if capability == Capability::Pointer && self.pointer.is_none() {
            println!("Set pointer capability");
            let pointer = self
                .seat_state
                .get_pointer(qh, &seat)
                .expect("Failed to create pointer");
            self.pointer = Some(pointer);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_some() {
            println!("Unset keyboard capability");
            self.keyboard.take().unwrap().release();
        }

        if capability == Capability::Pointer && self.pointer.is_some() {
            println!("Unset pointer capability");
            self.pointer.take().unwrap().release();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}
impl PointerHandler for WaylandClientState {
    fn pointer_frame(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        pointer: &WlPointer,
        events: &[smithay_client_toolkit::seat::pointer::PointerEvent],
    ) {
        if let Some(window) = self.window.clone() {
            for event in events {
                match event.kind {
                    PointerEventKind::Leave { serial } => {
                        window.handle_event(PlatformInput::MouseExited(crate::MouseExitEvent {
                            position: point(event.position.0.into(), event.position.1.into()),
                            pressed_button: self.pressed_button,
                            modifiers: self.modifiers,
                        }))
                    }
                    PointerEventKind::Enter { serial } => {}
                    PointerEventKind::Motion { time } => {
                        window.handle_event(PlatformInput::MouseMove(crate::MouseMoveEvent {
                            position: point(event.position.0.into(), event.position.1.into()),
                            pressed_button: self.pressed_button,
                            modifiers: self.modifiers,
                        }))
                    }
                    PointerEventKind::Press {
                        time,
                        button,
                        serial,
                    } => {
                        if let Some(button) = button_of_key(button) {
                            window.handle_event(PlatformInput::MouseDown(crate::MouseDownEvent {
                                position: point(event.position.0.into(), event.position.1.into()),
                                button,
                                modifiers: self.modifiers,
                                click_count: 1,
                            }));
                            self.pressed_button = Some(button);
                        }
                    }
                    PointerEventKind::Release {
                        time,
                        button,
                        serial,
                    } => {
                        if let Some(button) = button_of_key(button) {
                            window.handle_event(PlatformInput::MouseUp(crate::MouseUpEvent {
                                position: point(event.position.0.into(), event.position.1.into()),
                                button,
                                modifiers: self.modifiers,
                                click_count: 1,
                            }))
                        }
                        self.pressed_button = None;
                    }
                    PointerEventKind::Axis {
                        time,
                        horizontal,
                        vertical,
                        source,
                    } => {
                        let touch_phase = if horizontal.stop || vertical.stop {
                            self.scrolling = false;
                            TouchPhase::Ended
                        } else if self.scrolling {
                            TouchPhase::Moved
                        } else {
                            self.scrolling = true;
                            TouchPhase::Started
                        };
                        window.handle_event(PlatformInput::ScrollWheel(crate::ScrollWheelEvent {
                            position: point(event.position.0.into(), event.position.1.into()),
                            delta: ScrollDelta::Pixels(point(
                                horizontal.absolute.into(),
                                (-vertical.absolute).into(),
                            )),
                            modifiers: self.modifiers,
                            touch_phase,
                        }))
                    }
                }
            }
        }
    }
}
delegate_keyboard!(WaylandClientState);
delegate_pointer!(WaylandClientState);
delegate_seat!(WaylandClientState);
delegate_registry!(WaylandClientState);
impl ProvidesRegistryState for WaylandClientState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![SeatState,];
}
fn button_of_key(button: u32) -> Option<MouseButton> {
    Some(match button {
        272 => MouseButton::Left,
        274 => MouseButton::Middle,
        273 => MouseButton::Right,
        _ => return None,
    })
}
