use std::future::Future;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use winit::{
    event::{self, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};

#[allow(dead_code)]
pub enum ShaderStage {
    Vertex,
    Fragment,
    Compute,
}

mod gui;
use gui::Gui;
use eframe::App;

pub trait Example: 'static + Sized {
    fn optional_features() -> wgpu::Features {
        wgpu::Features::empty()
    }
    fn required_features() -> wgpu::Features {
        wgpu::Features::empty()
    }
    fn required_downlevel_capabilities() -> wgpu::DownlevelCapabilities {
        wgpu::DownlevelCapabilities {
            flags: wgpu::DownlevelFlags::empty(),
            shader_model: wgpu::ShaderModel::Sm5,
            ..wgpu::DownlevelCapabilities::default()
        }
    }
    fn required_limits() -> wgpu::Limits {
        wgpu::Limits::downlevel_webgl2_defaults() // These downlevel limits will allow the code to run on all possible hardware
    }
    fn init(
        config: &wgpu::SurfaceConfiguration,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Self;
    fn resize(
        &mut self,
        config: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    );
    fn update(&mut self, event: WindowEvent);
    fn render(
        &mut self,
        view: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        spawner: &Spawner,
    );
}

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

async fn setup<E: Example>(title: &str) -> Setup {

    //create main event loop for winit()
    let event_loop = EventLoop::new();
    
    //setup window
    let monitor = event_loop.primary_monitor().unwrap();
    let video_mode = monitor.video_modes().next();
    let size = video_mode.clone().map_or(winit::dpi::PhysicalSize::new(800, 600), |vm| vm.size());
    let mut builder = winit::window::WindowBuilder::new()
        .with_visible(true)
        .with_title("The universe, with a heck of a lot of rounding errors")
     //   .with_fullscreen(video_mode.map(|vm| winit::window::Fullscreen::Exclusive(vm)));
        .with_inner_size(winit::dpi::PhysicalSize {
            width: 1920, 
            height: 1080
        });
    let window = builder.build(&event_loop).unwrap();


    log::info!("Initializing the surface...");

    let backend = wgpu::util::backend_bits_from_env().unwrap_or_else(wgpu::Backends::all);

    let instance = wgpu::Instance::new(backend);
    let (size, surface) = unsafe {
        let size = window.inner_size();

        let surface = instance.create_surface(&window);

        (size, surface)
    };

    let adapter =
        wgpu::util::initialize_adapter_from_env_or_default(&instance, backend, Some(&surface))
            .await
            .expect("No suitable GPU adapters found on the system!");

    
    let adapter_info = adapter.get_info();
    println!("Using {} ({:?})", adapter_info.name, adapter_info.backend);
    

    let optional_features = E::optional_features();
    let required_features = E::required_features();
    let adapter_features = adapter.features();


    // Make sure we use the texture resolution limits from the adapter, so we can support images the size of the surface.
    let needed_limits = E::required_limits().using_resolution(adapter.limits());

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

fn start<E: Example>(
    Setup {
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
            format: surface.get_supported_formats(&adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Immediate,
            alpha_mode: surface.get_supported_alpha_modes(&adapter)[0],
    };
    surface.configure(&device, &config);
    

    //setup ui
    let mut platform = Platform::new(PlatformDescriptor {
        physical_width: size.width as u32,
        physical_height: size.height as u32,
        scale_factor: window.scale_factor(),
        font_definitions: egui::FontDefinitions::default(),
        style: Default::default(),
    });

    let mut egui_rpass = RenderPass::new(&device, config.format, 1);
    let mut test_ui = gui::Gui::default();

    let mut example = E::init(&config, &adapter, &device, &queue);

    let mut last_frame_inst = Instant::now();
    let (mut frame_count, mut accum_time) = (0, 0.0);

    //let start_time = Instant::now();
    event_loop.run(move |event, _, control_flow| {
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
                    println!("{:#?}", instance.generate_report());
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
                        println!(
                            "Avg frame time {}ms",
                            accum_time * 1000.0 / frame_count as f32
                        );
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
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                // //render ui
                // // Begin to draw the UI frame.
                // platform.begin_frame();

                // // Draw the demo application.
                // test_ui.ui(&platform.context());

                // // End the UI frame. We could now handle the output and draw the UI with the backend.
                // let full_output = platform.end_frame(Some(&window));
                // let paint_jobs = platform.context().tessellate(full_output.shapes);
                
                // //render UI on wgpu backend
                // let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                //     label: Some("encoder"),
                // });

                // // Upload all resources for the GPU.
                // let screen_descriptor = ScreenDescriptor {
                //     physical_width: size.width,
                //     physical_height: size.height,
                //     scale_factor: window.scale_factor() as f32,
                // };
                // let tdelta: egui::TexturesDelta = full_output.textures_delta;
                // egui_rpass
                //     .add_textures(&device, &queue, &tdelta)
                //     .expect("add texture ok");
                // egui_rpass.update_buffers(&device, &queue, &paint_jobs, &screen_descriptor);

                // // Record all render passes.
                // egui_rpass
                //     .execute(
                //         &mut encoder,
                //         &view,
                //         &paint_jobs,
                //         &screen_descriptor,
                //         Some(wgpu::Color::BLACK),
                //     )
                //     .unwrap();
                // // Submit the commands.
                // queue.submit(std::iter::once(encoder.finish()));

                
                example.render(&view, &device, &queue, &spawner);

                frame.present();

                // // Redraw egui
                // egui_rpass
                //     .remove_textures(tdelta)
                //     .expect("remove texture ok");

            }
            _ => {}
        }
    });
}


pub fn run<E: Example>(title: &str) {
    let setup = pollster::block_on(setup::<E>(title));
    start::<E>(setup);
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