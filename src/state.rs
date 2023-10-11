//use nanorand::{Rng, WyRand};
use std::{borrow::Cow, mem};

use eden::SAMPLE_COUNT;
use wgpu::{util::DeviceExt, TextureView};

#[derive(Debug)]
pub struct State {
    particle_bind_groups: Vec<wgpu::BindGroup>,
    pub active_particles: u32,
    pub particle_buffers: Vec<wgpu::Buffer>,
    circle_buffer: wgpu::Buffer,
    compute_pipeline: wgpu::ComputePipeline,
    render_pipeline: wgpu::RenderPipeline,
    work_group_count: u32,
    frame_num: usize,
    pub camera: eden::Camera,
    pub camera_uniform_buffer: wgpu::Buffer,
    pub params: eden::Params,
    camera_bind_group: wgpu::BindGroup,
    // post-processing stuff
   // tex_view: Option<wgpu::TextureView>,

}

impl State {
    pub fn required_limits() -> wgpu::Limits {
        //set surface limits based on the target architecture
        if cfg!(target_arch = "wasm32") {
            wgpu::Limits::downlevel_webgl2_defaults()
        } else {
            wgpu::Limits::default()
        }
    }

    pub fn required_downlevel_capabilities() -> wgpu::DownlevelCapabilities {
        //downlevel capabilites that don't confirm to WebGPU standard
        wgpu::DownlevelCapabilities {
            flags: wgpu::DownlevelFlags::COMPUTE_SHADERS,
            ..Default::default()
        }
    }

    pub fn init(
        params: eden::Params,
        config: &wgpu::SurfaceConfiguration,
        _adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) -> Self {

        //create parameters
        let params = params;
        let params_slice = params.to_slice();
        let params_attraction_matrix = params.attraction_matrix_slice();

        //initialize compute shader module
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&params.shader_buffer)),
        });

        //initialize vertex and fragment shaders
        let draw_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shaders/draw.wgsl"))),
        });

        //set up uniform buffer to store global parameters
        let sim_param_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Parameter Buffer"),
            contents: bytemuck::cast_slice(&params_slice),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        //set up uniform buffer to store global parameters
        let attraction_matrix_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Parameter Buffer"),
            contents: bytemuck::cast_slice(params_attraction_matrix),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        //set up camera buffer
       // let camera = Camera::new(1.0 / (params.world_size * 1.5));
       let aspect_ratio:f32 = config.width as f32 / config.height as f32;
       let camera = eden::Camera::new(1.0 / params.world_size, aspect_ratio);
        let camera_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&(camera.to_slice())),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        //set up compute bind group layouts and compute pipeline layours
        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    //PARAM buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (params_slice.len() * mem::size_of::<f32>()) as _,
                            ),
                        },
                        count: None,
                    },
                    //input / source buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (params.num_particles * 24) as _,
                            ), //CHANGE SIZE IF ISSUES
                        },
                        count: None,
                    },
                    //output / destination buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (params.num_particles * 24) as _,
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (params_attraction_matrix.len() * mem::size_of::<f32>()) as _,
                            ),
                        },
                        count: None,
                    },
                ],
                label: None,
            });
        //compute pipeline layout =
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            });

        //camera bind group layout
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        //render pipeline layout
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render"),
                bind_group_layouts: &[&camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        //initialize render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &draw_shader,
                entry_point: "main_vs",
                buffers: &[
                    //vertex buffer layout format (2 pos varibales, 2 vel variables)
                    wgpu::VertexBufferLayout {
                        array_stride:  24,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32, 3 => Float32],
                    },

                    wgpu::VertexBufferLayout {
                        array_stride: 2 * 4,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![4 => Float32x2],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &draw_shader,
                entry_point: "main_fs",
                targets: &[Some(config.format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // change to PointLIst
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: SAMPLE_COUNT,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        // create compute pipeline

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "main",
        });


        //buffer for particle circle coordinates

      //  let circle_buffer_data = [-0.01f32, -0.02, 0.01, -0.02, 0.00, 0.02];
        let circle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::bytes_of(&eden::generate_circle( 0.2)),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let mut initial_particle_data: Vec<f32>  = Vec::new();

        for _ in 0..params.num_particles {
            initial_particle_data.extend_from_slice(&eden::Particle::new_random(&params).to_slice())
        }

        // creates two buffers of particle data each of size NUM_PARTICLES
        // the two buffers alternate as dst and src for each frame

        let mut particle_buffers = Vec::<wgpu::Buffer>::new();
        let mut particle_bind_groups = Vec::<wgpu::BindGroup>::new();
        for i in 0..2 {
            particle_buffers.push(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("Particle Buffer {}", i)),
                    contents: bytemuck::cast_slice(&initial_particle_data),
                    usage: wgpu::BufferUsages::VERTEX
                        | wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_DST,
                }),
            );
        }

        //camera bind group
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_uniform_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        // create two bind groups, one for each buffer as the src
        // where the alternate buffer is used as the dst

        for i in 0..2 {
            particle_bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &compute_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: sim_param_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: particle_buffers[i].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: particle_buffers[(i + 1) % 2].as_entire_binding(), // bind to opposite buffer
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: attraction_matrix_buffer.as_entire_binding(), // bind to opposite buffer
                    },
                ],
                label: None,
            }));
        }

        // calculates number of work groups from PARTICLES_PER_GROUP constant
        let work_group_count = u32::min(params.num_particles, 65535);

        let active_particles: u32 = params.num_particles;
        // returns Example struct and No encoder commands

        //post-processing
        // let tex_view: wgpu::TextureView = device.create_texture()

        State {
            particle_bind_groups,
            active_particles,
            particle_buffers,
            circle_buffer,
            compute_pipeline,
            render_pipeline,
            work_group_count,
            frame_num: 0,
            camera,
            camera_uniform_buffer,
            params,
            camera_bind_group,
        }
    }

    /// update is called for any WindowEvent not handled by the framework
    pub fn update(&mut self, _event: winit::event::WindowEvent) {
        //empty
    }

    /// resize is called on WindowEvent::Resized events
    pub fn resize(
        &mut self,
        config: &wgpu::SurfaceConfiguration,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        self.camera.aspect_ratio = config.width as f32 / config.height as f32;
        //println!("New camera zoom: {:?}", example.camera.zoom);
        queue.write_buffer(&(self.camera_uniform_buffer), 0, bytemuck::cast_slice(&[self.camera.to_slice()]));

    }
    pub fn render(&mut self, view: &wgpu::TextureView, resolve_view: Option<&TextureView>, device: &wgpu::Device, queue: &wgpu::Queue) {
        // create render pass descriptor and its color attachments
        let color_attachments = [Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: resolve_view,
            ops: wgpu::Operations {
                // Not clearing here in order to test wgpu's zero texture initialization on a surface texture.
                // Users should avoid loading uninitialized memory since this can cause additional overhead.
                load: wgpu::LoadOp::Load,
                store: true,
            },
        })];
        let render_pass_descriptor = wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &color_attachments,
            depth_stencil_attachment: None,
        };

        // get command encoder
        let mut command_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        if self.params.play {
                // compute pass
                let mut cpass =
                    command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
                cpass.set_pipeline(&self.compute_pipeline);
                cpass.set_bind_group(0, &self.particle_bind_groups[self.frame_num % 2], &[]);
                cpass.dispatch_workgroups(self.work_group_count, 1, 1);

            } else {
                      self.frame_num -= 1;

        }


        //render pass
        {
            // render pass
            let mut rpass = command_encoder.begin_render_pass(&render_pass_descriptor);
            rpass.set_pipeline(&self.render_pipeline);
            //load camera uniform buffer
            rpass.set_bind_group(0, &self.camera_bind_group, &[]);
            // render dst particles
            rpass.set_vertex_buffer(0, self.particle_buffers[(self.frame_num + 1) % 2].slice(..));
            rpass.set_vertex_buffer(1, self.circle_buffer.slice(..));
            // the three instance-local vertices ????
            rpass.draw(0..((eden::CIRCLE_RES * 3) as u32), 0..self.params.num_particles);
        }

        // update frame count
        self.frame_num += 1;

        // done
        queue.submit(Some(command_encoder.finish()));
    }
    fn post_processing(&mut self, _view: &wgpu::TextureView, _device: &wgpu::Device, _queue: &wgpu::Queue ) {
        //light effect
        //
        // let render_pass_descriptor = wgpu::RenderPassDescriptor {
        //    label: None,
        //    color_attachments: &color_attachments,
        //  depth_stencil_attachment: None,
        // };


    }
}
