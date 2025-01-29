use std::convert::TryInto;

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
    shm::{
        slot::{Buffer, SlotPool},
        Shm, ShmHandler,
    },
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
    Connection, QueueHandle,
};

use crate::recognizer::Point;

pub fn get_user_gesture() -> Option<Vec<Point>> {
    let conn = Connection::connect_to_env().unwrap();

    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    let compositor = CompositorState::bind(&globals, &qh).expect("wl_compositor is not available");
    let layer_shell = LayerShell::bind(&globals, &qh).expect("layer shell is not available");
    let shm = Shm::bind(&globals, &qh).expect("wl_shm is not available");

    let pool = SlotPool::new(256 * 256 * 4, &shm).expect("Failed to create pool");

    let mut app = AppData {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        compositor,
        layer_shell,
        shm,

        state: AppState::Run,
        pool,
        layers: Vec::new(),
        keyboard: None,
        pointer: None,

        gesture_path: Vec::new(),
    };

    loop {
        match app.state {
            AppState::Run => {
                event_queue.blocking_dispatch(&mut app).unwrap();
            }
            AppState::Exit => {
                return None;
            }
            AppState::ExitRecognize => {
                return Some(app.gesture_path);
            }
        }
    }
}

enum AppState {
    Run,
    Exit,
    ExitRecognize,
}

struct AppData {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor: CompositorState,
    layer_shell: LayerShell,
    shm: Shm,

    state: AppState,
    pool: SlotPool,
    layers: Vec<OutputLayer>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,

    gesture_path: Vec<Point>,
}

struct OutputLayer {
    layer: LayerSurface,
    logical_size: (u32, u32),
    logical_position: (i32, i32),
    //pixels: ImageBuffer<Rgba<u8>, Vec<u8>>,
    buffer: Option<Buffer>,
}

impl CompositorHandler for AppData {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
        // Not needed for this example.
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
        // Not needed for this example.
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        //let Some(layer) = self
        //    .layers
        //    .iter_mut()
        //    .find(|l| *l.layer.wl_surface() == *surface)
        //else {
        //    return;
        //};
        //
        //let (width, height) = layer.logical_size;
        //let stride = width as i32 * 4;
        //
        //let buffer = layer.buffer.get_or_insert_with(|| {
        //    self.pool
        //        .create_buffer(
        //            width as i32,
        //            height as i32,
        //            stride,
        //            wl_shm::Format::Argb8888,
        //        )
        //        .expect("create buffer")
        //        .0
        //});
        //
        //let mut canvas = match self.pool.canvas(buffer) {
        //    Some(canvas) => canvas,
        //    None => {
        //        // This should be rare, but if the compositor has not released the previous
        //        // buffer, we need double-buffering.
        //        let (width, height) = layer.logical_size;
        //        let (second_buffer, canvas) = self
        //            .pool
        //            .create_buffer(
        //                width as i32,
        //                height as i32,
        //                stride,
        //                wl_shm::Format::Argb8888,
        //            )
        //            .expect("create buffer");
        //        *buffer = second_buffer;
        //        canvas
        //    }
        //};
        //
        //canvas.write_all(layer.pixels.as_bytes()).unwrap();
        //
        //let wl_surface = layer.layer.wl_surface();
        //
        //// Damage the entire window
        //wl_surface.damage_buffer(0, 0, width as i32, height as i32);
        //// Request our next frame
        //wl_surface.frame(qh, wl_surface.clone());
        //// Attach and commit to present.
        //buffer.attach_to(wl_surface).expect("buffer attach");
        //layer.layer.commit();
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for AppData {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        let info = self.output_state.info(&output).unwrap();

        let logical_size: (u32, u32) = {
            let (w, h) = info.logical_size.unwrap();
            (w.try_into().unwrap(), h.try_into().unwrap())
        };

        let logical_position: (i32, i32) = info.logical_position.unwrap();

        let surface = self.compositor.create_surface(&qh);
        let layer = self.layer_shell.create_layer_surface(
            &qh,
            surface,
            Layer::Overlay,
            Some("simple_layer"),
            Some(&output),
        );
        layer.set_anchor(Anchor::BOTTOM);
        layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
        layer.set_size(logical_size.0, logical_size.1);
        layer.commit();

        //let pixels: ImageBuffer<Rgba<u8>, Vec<u8>> =
        //    ImageBuffer::new(logical_size.0, logical_size.1);

        self.layers.push(OutputLayer {
            layer,
            logical_size,
            logical_position,
            //pixels,
            buffer: None,
        });
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for AppData {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        self.state = AppState::Exit;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let out_layer = self
            .layers
            .iter_mut()
            .find(|e| &e.layer == layer)
            .expect("failed to find related layer");

        let (width, height) = configure.new_size;

        if let Some(buffer) = out_layer.buffer.take() {
            buffer.wl_buffer().destroy();
        }

        let stride = width as i32 * 4;
        let (buffer, _canvas) = self
            .pool
            .create_buffer(
                width as i32,
                height as i32,
                stride,
                wl_shm::Format::Argb8888,
            )
            .expect("create buffer");

        buffer.attach_to(layer.wl_surface()).expect("buffer attach");

        out_layer.buffer = Some(buffer);

        layer
            .wl_surface()
            .damage_buffer(0, 0, width as i32, height as i32);
        layer.wl_surface().frame(qh, layer.wl_surface().clone());
        layer.commit();
    }
}

impl SeatHandler for AppData {
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
            let keyboard = self
                .seat_state
                .get_keyboard(qh, &seat, None)
                .expect("Failed to create keyboard");
            self.keyboard = Some(keyboard);
        }

        if capability == Capability::Pointer && self.pointer.is_none() {
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
        if capability == Capability::Keyboard {
            if let Some(keyboard) = self.keyboard.take() {
                keyboard.release();
            }
        }

        if capability == Capability::Pointer {
            if let Some(pointer) = self.pointer.take() {
                pointer.release();
            }
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl KeyboardHandler for AppData {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _keysyms: &[Keysym],
    ) {
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _: u32,
    ) {
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        _event: KeyEvent,
    ) {
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        if event.keysym == Keysym::Escape {
            self.state = AppState::Exit;
        }
        //self.state = match event.keysym {
        //    Keysym::Escape => AppState::Exit,
        //    _ => AppState::ExitRecognize,
        //}
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _modifiers: Modifiers,
        _layout: u32,
    ) {
        //if !modifiers.alt && !modifiers.shift && !modifiers.alt && !modifiers.logo {
        //    self.state = AppState::ExitRecognize;
        //}
    }
}

impl PointerHandler for AppData {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            if let PointerEventKind::Motion { .. } = event.kind {
                let Some(layer) = self
                    .layers
                    .iter_mut()
                    .find(|l| *l.layer.wl_surface() == event.surface)
                else {
                    continue;
                };

                let (x, y) = event.position;

                //draw_circle(
                //    &mut layer.pixels,
                //    x as i32,
                //    y as i32,
                //    5,
                //    Rgba::from([0xFF; 4]),
                //);

                //println!("mouse local position: x={}, y={}", x, y);
                let global_x = layer.logical_position.0 as f64 + x;
                let global_y = layer.logical_position.1 as f64 + y;
                //println!("mouse global position: x={}, y={}", global_x, global_y);

                self.gesture_path.push(Point::new(global_x, global_y));
            }

            if let PointerEventKind::Release { .. } = event.kind {
                self.state = AppState::ExitRecognize;
            }
        }
    }
}

impl ShmHandler for AppData {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

delegate_compositor!(AppData);
delegate_output!(AppData);
delegate_shm!(AppData);

delegate_seat!(AppData);
delegate_keyboard!(AppData);
delegate_pointer!(AppData);

delegate_layer!(AppData);

delegate_registry!(AppData);

impl ProvidesRegistryState for AppData {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

//fn draw_pattern(canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, path: &[Point], color: Rgba<u8>) {
//    for (index, point) in path.iter().enumerate() {
//        draw_circle(
//            canvas,
//            (point.x * 4.0) as i32 + 1500,
//            (point.y * 4.0) as i32 + 700,
//            (2 + 5 * index / path.len()) as i32,
//            color,
//        );
//    }
//}
//
//fn draw_circle(
//    canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
//    cx: i32,
//    cy: i32,
//    radius: i32,
//    color: Rgba<u8>,
//) {
//    let mut d = 3 - 2 * radius;
//    let (mut x, mut y) = (radius, 0);
//
//    while y <= x {
//        draw_line(canvas, cx - x, cy - y, cx + x, cy - y, color);
//        draw_line(canvas, cx - x, cy + y, cx + x, cy + y, color);
//        draw_line(canvas, cx - y, cy + x, cx + y, cy + x, color);
//        draw_line(canvas, cx - y, cy - x, cx + y, cy - x, color);
//
//        if d <= 0 {
//            d += 4 * y + 6;
//            y += 1;
//        } else {
//            d += 4 * (y - x) + 10;
//            x -= 1;
//            y += 1;
//        }
//    }
//}
//
//fn draw_line(
//    canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
//    mut x0: i32,
//    mut y0: i32,
//    mut x1: i32,
//    mut y1: i32,
//    color: Rgba<u8>,
//) {
//    let steep = (y1 - y0).abs() > (x1 - x0).abs();
//
//    if steep {
//        swap(&mut x0, &mut y0);
//        swap(&mut x1, &mut y1);
//    }
//
//    if x0 > x1 {
//        swap(&mut x0, &mut x1);
//        swap(&mut y0, &mut y1);
//    }
//
//    let delta_x = (x1 - x0).abs();
//    let delta_y = (y1 - y0).abs();
//    let mut error = delta_x / 2;
//    let mut y = y0;
//    let y_step = if y0 < y1 { 1 } else { -1 };
//
//    for x in x0..=x1 {
//        let (new_x, new_y) = if steep { (y, x) } else { (x, y) };
//
//        if canvas.in_bounds(new_x as u32, new_y as u32) {
//            canvas.put_pixel(new_x as u32, new_y as u32, color);
//        }
//
//        error -= delta_y;
//        if error < 0 {
//            y += y_step;
//            error += delta_x;
//        }
//    }
//}
//
//fn draw_rect(
//    canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
//    x: i32,
//    y: i32,
//    width: i32,
//    height: i32,
//    color: Rgba<u8>,
//) {
//    for x in x..x + width {
//        for y in y..y + height {
//            canvas.put_pixel(x as u32, y as u32, color);
//        }
//    }
//}
