use std::future::Future;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use eden::SAMPLE_COUNT;
use eden::TEXTURE_FORMAT;

use winit::{
    dpi::PhysicalPosition,
    event::{self, ElementState, MouseButton, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

#[allow(dead_code)]
pub fn cast_slice<T>(data: &[T]) -> &[u8] {
    use std::{mem::size_of, slice::from_raw_parts};

    unsafe { from_raw_parts(data.as_ptr() as *const u8, data.len() * size_of::<T>()) }
}

#[allow(dead_code)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}
use super::gui;
use super::state;

struct Setup {
    window: winit::window::Window,
    event_loop: EventLoop<()>,
    instance: wgpu::Instance,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

async fn setup(title: &str) -> Setup {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    };

    //create main event loop for winit()
    let event_loop = EventLoop::new();

    // let monitor = event_loop.primary_monitor().unwrap();
    // let video_mode = monitor.video_modes().next();
    // let size = video_mode.clone().map_or(winit::dpi::PhysicalSize::new(800, 600), |vm| vm.size());
    let builder = winit::window::WindowBuilder::new()
        .with_visible(true)
        .with_title("The universe, with a heck of a lot of rounding errors")
        //   .with_fullscreen(video_mode.map(|vm| winit::window::Fullscreen::Exclusive(vm)));
        .with_inner_size(winit::dpi::PhysicalSize {
            width: 1920,
            height: 1080,
        });

    #[cfg(windows_OFF)] // TODO
    {
        use winit::platform::windows::WindowBuilderExtWindows;
        builder = builder.with_no_redirection_bitmap(true);
    }
    let window = builder.build(&event_loop).unwrap();

    log::info!("Initializing the surface...");

    let backends = wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all);
    let dx12_shader_compiler = wgpu::util::dx12_shader_compiler_from_env().unwrap_or_default();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends,
        dx12_shader_compiler,
    });

    let (size, surface) = unsafe {
        let size = window.inner_size();

        let surface = instance.create_surface(&window).unwrap();

        (size, surface)
    };

    let adapter =
        wgpu::util::initialize_adapter_from_env_or_default(&instance, backends, Some(&surface))
            .await
            .expect("No suitable GPU adapters found on the system!");

    let adapter_info = adapter.get_info();
    println!("Using {} ({:?})", adapter_info.name, adapter_info.backend);

    let optional_features = wgpu::Features::empty();
    let required_features = wgpu::Features::empty();
    let adapter_features = adapter.features();
    // assert!(
    //     adapter_features.contains(required_features),
    //     "Adapter does not support required features for this example: {:?}",
    //     required_features - adapter_features
    // );

    // let required_downlevel_capabilities = E::required_downlevel_capabilities();
    // let downlevel_capabilities = adapter.get_downlevel_capabilities();
    // assert!(
    //     downlevel_capabilities.shader_model >= required_downlevel_capabilities.shader_model,
    //     "Adapter does not support the minimum shader model required to run this example: {:?}",
    //     required_downlevel_capabilities.shader_model
    // );
    // assert!(
    //     downlevel_capabilities
    //         .flags
    //         .contains(required_downlevel_capabilities.flags),
    //     "Adapter does not support the downlevel capabilities required to run this example: {:?}",
    //     required_downlevel_capabilities.flags - downlevel_capabilities.flags
    // );

    // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the surface.
    let needed_limits = state::State::required_limits().using_resolution(adapter.limits());

    let trace_dir = std::env::var("WGPU_TRACE");
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: (optional_features & adapter_features) | required_features,
                limits: needed_limits,
            },
            trace_dir.ok().as_ref().map(std::path::Path::new),
        )
        .await
        .expect("Unable to find a suitable GPU adapter!");

    Setup {
        window,
        event_loop,
        instance,
        size,
        surface,
        adapter,
        device,
        queue,
    }
}

fn start(
    #[cfg(not(target_arch = "wasm32"))] Setup {
        window,
        event_loop,
        instance,
        size,
        surface,
        adapter,
        device,
        queue,
    }: Setup,
) {
    let spawner = Spawner::new();
    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: TEXTURE_FORMAT,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: wgpu::CompositeAlphaMode::Auto, //Auto()
        view_formats: vec![TEXTURE_FORMAT],
    };
    surface.configure(&device, &config);

    let mut test_ui = gui::Gui::new(&window, &device, &config);
    log::info!("Initializing the example...");

    let params: eden::Params = eden::Params::new();

    let mut example = state::State::init(params, &config, &adapter, &device, &queue);

    let mut last_frame_inst = Instant::now();
    let (mut frame_count, mut accum_time) = (0, 0.0);
    let mut frame_rate: f32 = 0.0;

    log::info!("Entering render loop...");

    let mut mouseState: bool = false;
    let mut lastMousePosition: PhysicalPosition<f64> = PhysicalPosition { x: -1.0, y: -1.0 };

    //antialiasing
    //let mut smaa_target = SmaaTarget::new(&device, &queue, size.width.max(1), size.height.max(1), config.format, smaa::SmaaMode::Smaa1X);

    event_loop.run(move |event, _, control_flow| {
        test_ui.platform.handle_event(&event);
        let _ = (&instance, &adapter); // force ownership by the closure
                                       //platform.handle_event(&event); //ui handle event
        *control_flow = ControlFlow::Poll;
        match event {
            event::Event::RedrawEventsCleared => {
                spawner.run_until_stalled();
                window.request_redraw();
            }
            event::Event::WindowEvent {
                event:
                    WindowEvent::Resized(size)
                    | WindowEvent::ScaleFactorChanged {
                        new_inner_size: &mut size,
                        ..
                    },
                ..
            } => {
                config.width = size.width.max(1);
                config.height = size.height.max(1);
                example.resize(&config, &device, &queue);
                surface.configure(&device, &config);
            }
            event::Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput {
                    input:
                        event::KeyboardInput {
                            virtual_keycode: Some(event::VirtualKeyCode::Escape),
                            state: event::ElementState::Pressed,
                            ..
                        },
                    ..
                }
                | WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::KeyboardInput {
                    input:
                        event::KeyboardInput {
                            virtual_keycode: Some(event::VirtualKeyCode::R),
                            state: event::ElementState::Pressed,
                            ..
                        },
                    ..
                } => {
                    let params = test_ui.gen_params();
                    example = state::State::init(params, &config, &adapter, &device, &queue);
                }

                WindowEvent::MouseWheel { delta, .. } => {
                    match delta {
                        event::MouseScrollDelta::LineDelta(x, y) => {
                            example.camera.zoom *= f32::powf(1.25, y);
                            //println!("New camera zoom: {:?}", example.camera.zoom);
                            queue.write_buffer(
                                &(example.camera_uniform_buffer),
                                0,
                                bytemuck::cast_slice(&[example.camera.to_slice()]),
                            );
                        }
                        _ => {}
                    }
                }
                WindowEvent::MouseInput {
                    device_id,
                    state,
                    button,
                    modifiers,
                } => {
                    match button {
                        MouseButton::Right => match state {
                            ElementState::Pressed => {
                                mouseState = true;
                            }
                            ElementState::Released => {
                                mouseState = false;
                                lastMousePosition = PhysicalPosition::<f64> { x: -1.0, y: 0.0 };
                            }
                        },
                        MouseButton::Left => {
                            //queue.write_buffer(&(example.particle_buffers[0]), 24 as u64, bytemuck::cast_slice(&[eden::Particle::new().to_slice()]));
                        }
                        _ => {}
                    }
                }
                WindowEvent::CursorMoved {
                    device_id,
                    position,
                    modifiers,
                } => {
                    if (mouseState) {
                        if (lastMousePosition.x != -1.0) {
                            let deltaPosition = PhysicalPosition::<f64> {
                                x: (position.x - lastMousePosition.x) / (config.width as f64),
                                y: (position.y - lastMousePosition.y) / (config.height as f64),
                            };

                            example.camera.x -=
                                (deltaPosition.x as f32 / example.camera.zoom) * 2.0;
                            example.camera.y +=
                                (deltaPosition.y as f32 / example.camera.zoom) * 2.0;

                            queue.write_buffer(
                                &(example.camera_uniform_buffer),
                                0,
                                bytemuck::cast_slice(&[example.camera.to_slice()]),
                            );

                            lastMousePosition = position;
                        } else {
                            lastMousePosition = position;
                        }
                    }
                }
                _ => {
                    example.update(event);
                }
            },
            event::Event::RedrawRequested(_) => {
                //platform.update_time(start_time.elapsed().as_secs_f64());
                {
                    accum_time += last_frame_inst.elapsed().as_secs_f32();
                    last_frame_inst = Instant::now();
                    frame_count += 1;
                    if frame_count == 100 {
                        test_ui.frame_rate = accum_time * 1000.0 / frame_count as f32;

                        // println!(
                        //     "Avg frame time {}ms",
                        //     accum_time * 1000.0 / frame_count as f32
                        // );
                        accum_time = 0.0;
                        frame_count = 0;
                    }
                }

                let frame = match surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(_) => {
                        surface.configure(&device, &config);
                        surface
                            .get_current_texture()
                            .expect("Failed to acquire next surface texture!")
                    }
                };

                let depthframe = device.create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width: config.width,
                        height: config.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: SAMPLE_COUNT,
                    dimension: wgpu::TextureDimension::D2,
                    format: TEXTURE_FORMAT,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                });

                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                //render ui

                let msaaview = depthframe.create_view(&wgpu::TextureViewDescriptor::default());

                match test_ui.state {
                    gui::OutputState::ReloadRequired => {
                        let params = test_ui.gen_params();
                        example = state::State::init(params, &config, &adapter, &device, &queue);
                    }
                    gui::OutputState::TogglePlay => {
                        example.params.play = !(example.params.play);
                    }
                    gui::OutputState::Debug => {
                        example.debug(&device, &queue);
                    }
                    gui::OutputState::Step => {
                        if (SAMPLE_COUNT == 1) {
                            example.render(&view, None, &device, &queue, true);
                        } else {
                            example.render(&msaaview, Some(&view), &device, &queue, true);
                        }
                    }
                    gui::OutputState::None => {
                        if (SAMPLE_COUNT == 1) {
                            example.render(&view, None, &device, &queue, example.params.play);
                        } else {
                            example.render(
                                &msaaview,
                                Some(&view),
                                &device,
                                &queue,
                                example.params.play,
                            );
                        }
                    }
                }

                test_ui.render(&window, &device, &view, &queue);

                frame.present();

                test_ui.cleanup();

                if (test_ui.state == gui::OutputState::ReloadRequired) {}
                #[cfg(target_arch = "wasm32")]
                {
                    if let Some(offscreen_canvas_setup) = &offscreen_canvas_setup {
                        let image_bitmap = offscreen_canvas_setup
                            .offscreen_canvas
                            .transfer_to_image_bitmap()
                            .expect("couldn't transfer offscreen canvas to image bitmap.");
                        offscreen_canvas_setup
                            .bitmap_renderer
                            .transfer_from_image_bitmap(&image_bitmap);
                    }
                }
            }
            _ => {}
        }
    });
}

pub struct Spawner<'a> {
    executor: async_executor::LocalExecutor<'a>,
}

impl<'a> Spawner<'a> {
    fn new() -> Self {
        Self {
            executor: async_executor::LocalExecutor::new(),
        }
    }

    #[allow(dead_code)]
    pub fn spawn_local(&self, future: impl Future<Output = ()> + 'a) {
        self.executor.spawn(future).detach();
    }

    fn run_until_stalled(&self) {
        while self.executor.try_tick() {}
    }
}

#[cfg(target_arch = "wasm32")]
pub struct Spawner {}

#[cfg(target_arch = "wasm32")]
impl Spawner {
    fn new() -> Self {
        Self {}
    }

    #[allow(dead_code)]
    pub fn spawn_local(&self, future: impl Future<Output = ()> + 'static) {
        wasm_bindgen_futures::spawn_local(future);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run(title: &str) {
    let setup = pollster::block_on(setup(title));
    start(setup);
}

#[cfg(target_arch = "wasm32")]
pub fn run(title: &str) {
    use wasm_bindgen::{prelude::*, JsCast};

    let title = title.to_owned();
    wasm_bindgen_futures::spawn_local(async move {
        let setup = setup(&title).await;
        let start_closure = Closure::once_into_js(move || start(setup));

        // make sure to handle JS exceptions thrown inside start.
        // Otherwise wasm_bindgen_futures Queue would break and never handle any tasks again.
        // This is required, because winit uses JS exception for control flow to escape from `run`.
        if let Err(error) = call_catch(&start_closure) {
            let is_control_flow_exception = error.dyn_ref::<js_sys::Error>().map_or(false, |e| {
                e.message().includes("Using exceptions for control flow", 0)
            });

            if !is_control_flow_exception {
                web_sys::console::error_1(&error);
            }
        }

        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(catch, js_namespace = Function, js_name = "prototype.call.call")]
            fn call_catch(this: &JsValue) -> Result<(), JsValue>;
        }
    });
}

#[cfg(target_arch = "wasm32")]
/// Parse the query string as returned by `web_sys::window()?.location().search()?` and get a
/// specific key out of it.
pub fn parse_url_query_string<'a>(query: &'a str, search_key: &str) -> Option<&'a str> {
    let query_string = query.strip_prefix('?')?;

    for pair in query_string.split('&') {
        let mut pair = pair.split('=');
        let key = pair.next()?;
        let value = pair.next()?;

        if key == search_key {
            return Some(value);
        }
    }

    None
}

// This allows treating the framework as a standalone example,
// thus avoiding listing the example names in `Cargo.toml`.
//#[allow(dead_code)]
// fn main() {}
