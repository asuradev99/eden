use egui;

use egui_winit_platform::{Platform, PlatformDescriptor};
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};

use winit::window::Window;

use wgpu::{Device, SurfaceConfiguration, TextureView};
#[derive(PartialEq)]
pub enum OutputState {
    ReloadRequired, 
    None,
}
pub struct Gui {
    pub platform: Platform,
    egui_rpass: egui_wgpu_backend::RenderPass,
    tdelta: egui::TexturesDelta,
    pub state: OutputState,
}

impl Gui {
    pub fn new(window: &Window, device: &Device, config: &SurfaceConfiguration) -> Self {
        let egui_rpass = RenderPass::new(&device, config.format, 1);
        let platform = Platform::new(PlatformDescriptor {
            physical_width: window.inner_size().width as u32,
            physical_height: window.inner_size().height as u32,
            scale_factor: window.scale_factor(),
            font_definitions: egui::FontDefinitions::default(),
            style: Default::default(),
        }); 
        let tdelta = egui::TexturesDelta::default();
        let state = OutputState::None;
        Self {
           platform, 
           egui_rpass,
           tdelta,
           state
        }
    }
    pub fn ui(&mut self) {
        // egui::CentralPanel::default().show(&self.platform.context(), |ui| {
        //     ui.heading("My egui Application");
        //     ui.horizontal(|ui| {
        //         ui.label("Your name: s ");
        //     });
        // });
        let mut open = true;
        self.state = OutputState::None;
        egui::Window::new("📤 Output Events")
            .open(&mut open)
            .resizable(true)
            .default_width(520.0)
            .show(&self.platform.context(), |ui| {
                ui.label(
                    "Recent output events from egui. \
            These are emitted when you interact with widgets, or move focus between them with TAB. \
            They can be hooked up to a screen reader on supported platforms.",
                );

                ui.separator();
                if ui.add(egui::Button::new("Click me")).clicked() {
                    self.state = OutputState::ReloadRequired;
                }
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        
                    });
            });
    }

    pub fn render(&mut self, window: &Window, device: &Device, view: &TextureView, queue: &wgpu::Queue) {
        // Begin to draw the UI frame.
        self.platform.begin_frame();

        // Draw the demo application.
        self.ui();

        // End the UI frame. We could now handle the output and draw the UI with the backend.
        let full_output = self.platform.end_frame(Some(&window));
        let paint_jobs = self.platform.context().tessellate(full_output.shapes);

        //render UI on wgpu backend
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("encoder"),
        });

        // Upload all resources for the GPU.
        let screen_descriptor = ScreenDescriptor {
            physical_width: window.inner_size().width,
            physical_height: window.inner_size().height,
            scale_factor: window.scale_factor() as f32,
        };
        self.tdelta = full_output.textures_delta;
        self.egui_rpass
            .add_textures(&device, &queue, &self.tdelta)
            .expect("add texture ok");
        self.egui_rpass.update_buffers(&device, &queue, &paint_jobs, &screen_descriptor);

        // Record all render passes.
        self.egui_rpass
            .execute(
                &mut encoder,
                &view,
                &paint_jobs,
                &screen_descriptor,
                None,
            )
            .unwrap();
        // Submit the commands.
        queue.submit(std::iter::once(encoder.finish()));
    }

    pub fn cleanup(&mut self) {
        // Redraw egui
        self.egui_rpass
        .remove_textures(self.tdelta.clone())
        .expect("remove texture ok");
    }

}