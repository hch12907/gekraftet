use std::time::Instant;

mod camera;
mod input;
mod mesh;
mod renderer;
mod windowing;
mod world;

use cgmath::*;
use gekraftet_core::world::*;
use camera::*;
use input::*;
use renderer::*;
use windowing::*;
use world::Mesher;

pub type RGBA = cgmath::Vector4<f32>;

fn main() {
    let w = Window::create_window();
    let mut r = GlRenderer::new(&w, 
        cgmath::perspective(Deg(55.0), 16.0/9.0, 0.1, 500.0)
    );

    let (tx, rx) = std::sync::mpsc::channel::<(i32, i32, i32, mesh::Mesh)>();
    let (bound0, bound1) = (-16i32, 16i32);

    let world_minister = std::thread::spawn(move || {
        let tx = tx;
        
        for x in bound0..bound1 {
            for y in bound0..bound1 {
                let tx = tx.clone();
                let mut noise = Noise::<Perlin3D>::with_option(
                    NoiseGenOption::new()
                        .octaves(16)
                        .amplitude(10.0)
                        .persistance(0.5)
                        .frequency(628.318530)
                        .lacunarity(0.5),
                    ((x << 6) ^ (y + 123456)) as u64,
                );

                std::thread::spawn(move || {
                    let pos = Point3::<i32>::new(x, 0, y);
                    let chunk = Chunk::new(pos, &mut noise);
                    let mesher = world::GreedyCubeMesher::from_chunk(&chunk);
                    let mesh = mesher.generate_mesh();
                    tx.send((pos.x, pos.y, pos.z, mesh))
                });
            }
        }

        drop(tx);
    });
    
    let speed = 10.0;

    let mut mouse_locked = false;
    let mut pos = Point3::<f32>::new(0.0, 200.0, 0.0);

    let mut cam = Camera::new(pos, Vector3::<f32>::new(2.5, -200.0, 0.5));
    let mut input_manager = InputManager::new();

    let mut last_time = Instant::now();
    let mut delta = 0.0;
    let mut time = 0.0;

    w.run(move |event, cl, context| {
        use glutin::window::CursorGrabMode;
        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CloseRequested => {
                        *cl = ControlFlow::Exit;
                    },
                    
                    WindowEvent::MouseInput { button, .. } => {
                        use glutin::event::MouseButton;
                        match button {
                            MouseButton::Left => {
                                context.window()
                                    .set_cursor_grab(CursorGrabMode::Locked)
                                    .expect("unable to grab cursor");
                                context.window()
                                    .set_cursor_visible(false);
                                mouse_locked = true;
                                input_manager.unsuspend_input();
                            },
                            _ => {}
                        }
                    },

                    WindowEvent::Resized(glutin::dpi::PhysicalSize::<u32> { width, height }) => 
                        r.change_viewport(width, height),

                    _ => {}
                }
            },

            Event::MainEventsCleared => {
                let mut new_speed = speed;
                let sensitivity = cam.sensitivity();
                let up = Vector3::<f32>::new(0.0, 1.0, 0.0);

                cam.move_camera(pos);

                if let Ok((x, y , z, mesh)) = rx.recv() {
                    println!(
                        "chunk at ({}, {}, {}) has {} vertices and {} indices",
                        x, y, z,
                        mesh.vertices().len(),
                        mesh.indices().len(),
                    );
                    r.render_mesh(mesh);
                }

                // Prioritise modifiers like LShift.
                for key in input_manager.iterate_held_keys() {
                    match key {
                        &Key::LShift => new_speed *= 2.0,
                        &Key::LControl => new_speed *= 0.2,
                        _ => {}
                    }
                }

                for key in input_manager.iterate_held_keys() {
                    match key {
                        &Key::W => pos += new_speed * delta * cam.front(),
                        &Key::S => pos -= new_speed * delta * cam.front(),
                        //&Key::W => pos += maths::Matrix3::rotate_y_axis(maths::Deg(-90.0)) * (new_speed * delta * cam.front().cross(up).normalize()),
                        //&Key::S => pos -= maths::Matrix3::rotate_y_axis(maths::Deg(-90.0)) * (new_speed * delta * cam.front().cross(up).normalize()),
                        &Key::A => pos -= new_speed * delta * cam.front().cross(up).normalize(),
                        &Key::D => pos += new_speed * delta * cam.front().cross(up).normalize(),

                        &Key::Escape => {
                            context.window()
                                .set_cursor_grab(CursorGrabMode::None)
                                .expect("unable to ungrab cursor");
                            context.window()
                                .set_cursor_visible(true);
                            mouse_locked = false;
                        },
                        _ => {}
                    }
                }

                if input_manager.is_key_pressed(Key::Equals) {
                    cam.set_sensitivity(sensitivity + 0.05)
                }

                if input_manager.is_key_pressed(Key::Minus) {
                    cam.set_sensitivity(sensitivity - 0.05)
                }

                if input_manager.is_key_pressed(Key::E) {
                    println!("{:?}", pos * 4.0);
                }

                if !mouse_locked {
                    input_manager.suspend_input();
                }

                let (delta_x, delta_y) = input_manager.get_mouse_delta(); {
                    cam.rotate_by_mouse(delta_x as f32, delta_y as f32, delta);
                };

                context.window().request_redraw();
            },

            Event::DeviceEvent { device_id, event, .. } => {
                input_manager.update_inputs(device_id, event);
            }

            Event::RedrawRequested(_id) => {
                r.render(time, cam.generate_view());

                time += 1.0;
                std::thread::sleep(std::time::Duration::from_micros(4167/*16667*/));
                context.swap_buffers().unwrap();
                let now = Instant::now();
                delta = (now - last_time).as_secs_f32();
                last_time = now;
            },

            _ => {
                // do nothing
            }
        };
    });
}
