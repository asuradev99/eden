use egui::{self};

use std::fs;

use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};

use glob::glob;
use winit::window::Window;

use wgpu::{Device, SurfaceConfiguration, TextureView};
#[derive(PartialEq)]
pub enum OutputState {
    ReloadRequired,
    TogglePlay,
    Debug,
    None,
    Step,
}
use eden::Params;

use eden::TEXTURE_FORMAT;

pub struct Gui {
    pub platform: Platform,
    egui_rpass: egui_wgpu_backend::RenderPass,
    tdelta: egui::TexturesDelta,
    pub state: OutputState,
    inner_params: Params,
    pub frame_rate: f32,
    pub shader_options: Vec<String>,
    pub selected_shader_file: String,
}

impl Gui {
    pub fn new(window: &Window, device: &Device, _config: &SurfaceConfiguration) -> Self {
        let egui_rpass = RenderPass::new(device, TEXTURE_FORMAT, 1);
        let platform = Platform::new(PlatformDescriptor {
            physical_width: window.inner_size().width as u32,
            physical_height: window.inner_size().height as u32,
            scale_factor: window.scale_factor(),
            font_definitions: egui::FontDefinitions::default(),
            style: Default::default(),
        });
        let tdelta = egui::TexturesDelta::default();
        let state = OutputState::None;
        let inner_params = Params::new();
        let frame_rate = 0.0;

        let mut shader_options: Vec<String> = Vec::new();
        let selected_shader_file = String::from("experimental.wgsl");

        for entry in glob("./shaders/*").expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    shader_options.push(String::from(path.file_name().unwrap().to_str().unwrap()))
                }
                Err(e) => println!("{:?}", e),
            }
        }

        Self {
            platform,
            egui_rpass,
            tdelta,
            state,
            inner_params,
            frame_rate,
            shader_options,
            selected_shader_file,
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
        egui::Window::new("Simulation Parameters")
            .open(&mut open)
            .resizable(true)
            .default_width(520.0)
            .show(&self.platform.context(), |ui| {
                egui::Grid::new("my_grid")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.style_mut().spacing.slider_width = 30.0;

                        ui.label(format!("Frame Rate: {}", self.frame_rate));
                        ui.end_row();

                        ui.label("World Size: ");
                        ui.add(egui::DragValue::new(&mut self.inner_params.world_size));
                        ui.end_row();

                        ui.label("Grid Lengths Per Side: ");
                        ui.add(egui::DragValue::new(&mut self.inner_params.num_grids_side));
                        ui.end_row();

                        ui.label("Number of Types: ");
                        ui.add(egui::DragValue::new(&mut self.inner_params.num_types));
                        ui.end_row();

                        let max_types: usize = ((self.inner_params.attraction_matrix.len() / 4)
                            as f32)
                            .sqrt() as usize;
                        for i in 0..max_types {
                            ui.label(format!("Particle Type {} Forces: ", i));
                            ui.horizontal(|ui| {
                                for j in 0..max_types {
                                    ui.add(
                                        egui::Slider::new(
                                            &mut self.inner_params.attraction_matrix
                                                [i * 4 * max_types + j * 4],
                                            -1.0..=1.0,
                                        )
                                        .show_value(false),
                                    );
                                }
                            });

                            ui.end_row();
                        }

                        ui.label("Delta Time: ");
                        ui.add(egui::DragValue::new(&mut self.inner_params.dt).max_decimals(5));
                        ui.end_row();

                        ui.label("Number of Particles: ");
                        ui.add(egui::DragValue::new(&mut self.inner_params.num_particles));
                        ui.end_row();

                        ui.label("(Lennard-Jones) Well Depth: ");
                        ui.add(egui::DragValue::new(&mut self.inner_params.well_depth));
                        ui.end_row();

                        ui.label("(Lennard-Jones): Attraction Coefficient");
                        ui.add(egui::Slider::new(
                            &mut self.inner_params.attract_coeff,
                            0.0..=1.0,
                        ));
                        ui.end_row();

                        ui.label("(Lennard-Jones): Repulsion Coefficient");
                        ui.add(egui::Slider::new(
                            &mut self.inner_params.repulse_coeff,
                            0.0..=1.0,
                        ));
                        ui.end_row();

                        ui.label(" Friction Coefficient");
                        ui.add(egui::Slider::new(
                            &mut self.inner_params.friction_coeff,
                            0.0..=1.0,
                        ));
                        ui.end_row();

                        ui.label("Particle Radius");
                        ui.add(egui::Slider::new(
                            &mut self.inner_params.particle_radius,
                            0.0..=1.0,
                        ));

                        ui.end_row();
                        ui.label("Compute Shader File");
                        egui::ComboBox::from_label("")
                            .selected_text(format!("{}", self.selected_shader_file))
                            .show_ui(ui, |ui| {
                                for file in &self.shader_options {
                                    if ui
                                        .add(egui::SelectableLabel::new(
                                            self.selected_shader_file == *file,
                                            file,
                                        ))
                                        .clicked()
                                    {
                                        self.selected_shader_file = file.clone();
                                        let filename = String::from("shaders/")
                                            + &self.selected_shader_file.clone();
                                        self.inner_params.shader_buffer =
                                            fs::read_to_string(&filename[..]).unwrap();
                                        println!("{:?}", self.inner_params.shader_buffer);
                                    }
                                }
                            });

                        ui.end_row();

                        egui::widgets::color_picker::color_edit_button_rgb(
                            ui,
                            &mut [255.0, 0.0, 0.0],
                        );
                    });

                if ui.add(egui::Button::new("Restart Simulation")).clicked() {
                    self.state = OutputState::ReloadRequired;
                }

                if ui
                    .add(egui::Button::new("Randomize Attraction Matrix"))
                    .clicked()
                {
                    self.inner_params.randomize_matrix();
                }
                if ui.add(egui::Button::new("Play / Pause")).clicked() {
                    self.state = OutputState::TogglePlay;
                }
                if ui.add(egui::Button::new("Debug")).clicked() {
                    //download
                    //
                    self.state = OutputState::Debug;
                }
                if ui.add(egui::Button::new("Step")).clicked() {
                    self.state = OutputState::Step;
                }
            });

        //  egui::Window::new("Edit Shader")
        //  .resizable(true)
        //   .min_width(10000.0)
        // .fixed_size([500.0, 500.0])
        // .anchor(egui::Align2::RIGHT_TOP, [-5.0, 5.0])
        // .show(&self.platform.context(), |ui| {
        //   ScrollArea::vertical().show(ui, |ui| {
        //     ui.add(TextEdit::multiline(&mut self.inner_params.shader_buffer).code_editor());
        // });
        // });
    }

    pub fn render(
        &mut self,
        window: &Window,
        device: &Device,
        view: &TextureView,
        queue: &wgpu::Queue,
    ) {
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
        self.egui_rpass
            .update_buffers(&device, &queue, &paint_jobs, &screen_descriptor);

        // Record all render passes.
        self.egui_rpass
            .execute(&mut encoder, &view, &paint_jobs, &screen_descriptor, None)
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

    pub fn gen_params(&self) -> Params {
        self.inner_params.clone()
    }
}
