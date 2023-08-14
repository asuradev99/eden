//use nanorand::{Rng, WyRand};
use std::{borrow::Cow, mem};

use eden::{Particle, SAMPLE_COUNT};
use eframe::Result;
use wgpu::{util::DeviceExt, TextureView};

#[derive(Debug)]
pub struct State {
    particle_bind_groups: Vec<wgpu::BindGroup>,
    preprocessing_bind_groups: Vec<wgpu::BindGroup>,
    cleanup_bind_groups: Vec<wgpu::BindGroup>,
    pub active_particles: u32,
    pub particle_buffers: Vec<wgpu::Buffer>,
    pub bucket_indeces_buffer: wgpu::Buffer,
    circle_buffer: wgpu::Buffer,
    compute_pipeline: wgpu::ComputePipeline,
    preprocessing_pipeline: wgpu::ComputePipeline,
    cleanup_pipeline: wgpu::ComputePipeline,
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

        //create bucket index buffer data

        let bucket_indeces_data: Vec<i32> = vec![-1; params.num_grids_side.pow(2) as usize];

        //nitialize preprocessing shader
        let preprocessing_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Preprocessing Shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                "shaders/preprocessing.wgsl"
            ))),
        });

        //initialize compute shader module
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&params.shader_buffer)),
        });

        let cleanup_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shaders/cleanup.wgsl"))),
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
        let attraction_matrix_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Parameter Buffer"),
                contents: bytemuck::cast_slice(params_attraction_matrix),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let bucket_indeces_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Bucket Indeces"),
            contents: bytemuck::cast_slice(&bucket_indeces_data),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

        //set up camera buffer
        // let camera = Camera::new(1.0 / (params.world_size * 1.5));
        let aspect_ratio: f32 = config.width as f32 / config.height as f32;
        let camera = eden::Camera::new(
            params.world_size / 4.0,
            params.world_size / 4.0,
            1.0 / params.world_size,
            aspect_ratio,
        );
        let camera_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&(camera.to_slice())),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let preprocessing_bind_group_layout =
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
                                (params.num_particles * (mem::size_of::<Particle>() as u32)) as _,
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
                                (params.num_particles * (mem::size_of::<Particle>() as u32)) as _,
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                bucket_indeces_data.len() as u64
                                    * std::mem::size_of::<i32>() as u64,
                            ),
                        },
                        count: None,
                    },
                ],
                label: Some("Preprocessing Bind Group Layout"),
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
                                (params.num_particles * (mem::size_of::<Particle>() as u32)) as _,
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
                                (params.num_particles * (mem::size_of::<Particle>() as u32)) as _,
                            ),
                        },
                        count: None,
                    },
                    //attraction matrix buffer
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
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                bucket_indeces_data.len() as u64 * mem::size_of::<i32>() as u64,
                            ),
                        },
                        count: None,
                    },
                ],
                label: None,
            });

        let cleanup_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    //input / source buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (params.num_particles * (mem::size_of::<Particle>() as u32)) as _,
                            ), //CHANGE SIZE IF ISSUES
                        },
                        count: None,
                    },
                    //output / destination buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                (params.num_particles * (mem::size_of::<Particle>() as u32)) as _,
                            ),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(
                                bucket_indeces_data.len() as u64
                                    * std::mem::size_of::<i32>() as u64,
                            ),
                        },
                        count: None,
                    },
                ],
                label: Some("Cleanup Bind Group Layout"),
            });

        let preprocessing_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Preprocessing"),
                bind_group_layouts: &[&preprocessing_bind_group_layout],
                push_constant_ranges: &[],
            });
        //compute pipeline layout =
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            });

        let cleanup_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Preprocessing"),
                bind_group_layouts: &[&cleanup_group_layout],
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
                        array_stride:  mem::size_of::<Particle>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32, 3 => Float32],
                    },

                    //coordinates to draw circle

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
                count: 4,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        // create compute pipeline
        let preprocessing_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Preprocessing Pipeline"),
                layout: Some(&preprocessing_pipeline_layout),
                module: &preprocessing_shader,
                entry_point: "main",
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "main",
        });

        // create compute pipeline
        let cleanup_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Cleanup Pipeline"),
            layout: Some(&cleanup_pipeline_layout),
            module: &cleanup_shader,
            entry_point: "main",
        });

        //buffer for particle circle coordinates

        //  let circle_buffer_data = [-0.01f32, -0.02, 0.01, -0.02, 0.00, 0.02];
        let circle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::bytes_of(&eden::generate_circle(params.particle_radius)),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let mut initial_particle_data: Vec<f32> = Vec::new();

        for _ in 0..params.num_particles {
            initial_particle_data.extend_from_slice(&eden::Particle::new_random(&params).to_slice())
        }

        // creates two buffers of particle data each of size NUM_PARTICLES
        // the two buffers alternate as dst and src for each frame

        let mut particle_buffers = Vec::<wgpu::Buffer>::new();
        let mut preprocessing_bind_groups = Vec::<wgpu::BindGroup>::new();
        let mut particle_bind_groups = Vec::<wgpu::BindGroup>::new();

        let mut cleanup_bind_groups = Vec::<wgpu::BindGroup>::new();

        for i in 0..2 {
            particle_buffers.push(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("Particle Buffer {}", i)),
                    contents: bytemuck::cast_slice(&initial_particle_data),
                    usage: wgpu::BufferUsages::VERTEX
                        | wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_DST
                        | wgpu::BufferUsages::COPY_SRC,
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

        preprocessing_bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &preprocessing_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: sim_param_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: particle_buffers[0].as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: particle_buffers[1].as_entire_binding(), // bind to opposite buffer
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: bucket_indeces_buffer.as_entire_binding(),
                },
            ],
            label: None,
        }));

        particle_bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: sim_param_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: particle_buffers[1].as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: particle_buffers[0].as_entire_binding(), // bind to opposite buffer
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: attraction_matrix_buffer.as_entire_binding(), // bind to opposite buffer
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: bucket_indeces_buffer.as_entire_binding(),
                },
            ],
            label: None,
        }));
        //
        // cleanup_bind_groups.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     layout: &cleanup_group_layout,
        //     entries: &[
        //         wgpu::BindGroupEntry {
        //             binding: 0,
        //             resource: particle_buffers[2].as_entire_binding(),
        //         },
        //         wgpu::BindGroupEntry {
        //             binding: 1,
        //             resource: particle_buffers[0].as_entire_binding(),
        //         },
        //         wgpu::BindGroupEntry {
        //             binding: 2,
        //             resource: bucket_indeces_buffer.as_entire_binding(),
        //         },
        //     ],
        //     label: None,
        // }));

        // calculates number of work groups from PARTICLES_PER_GROUP constant
        let work_group_count = u32::min(params.num_particles, 65535);

        let active_particles: u32 = params.num_particles;
        // returns Example struct and No encoder commands

        //post-processing
        // let tex_view: wgpu::TextureView = device.create_texture()

        State {
            particle_bind_groups,
            preprocessing_bind_groups,
            cleanup_bind_groups,
            active_particles,
            particle_buffers,
            bucket_indeces_buffer,
            circle_buffer,
            compute_pipeline,
            preprocessing_pipeline,
            render_pipeline,
            cleanup_pipeline,
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
        queue.write_buffer(
            &(self.camera_uniform_buffer),
            0,
            bytemuck::cast_slice(&[self.camera.to_slice()]),
        );
    }

    pub fn debug(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
        use eden::Particle;
        use std::mem;
        use std::result;

        let with_buffer =
            |result: result::Result<wgpu::util::DownloadBuffer, wgpu::BufferAsyncError>| {
                let buffer: &[u8] = &*result.unwrap();
                let mut particle_buffer: Vec<Particle> = Vec::new();

                for particle_bytes in buffer.chunks_exact(std::mem::size_of::<Particle>()) {
                    let bytes_fixed: *const [u8; std::mem::size_of::<Particle>()] =
                        particle_bytes.as_ptr() as *const [u8; std::mem::size_of::<Particle>()];
                    unsafe {
                        let new_particle: Particle = mem::transmute(*bytes_fixed);
                        particle_buffer.push(new_particle);
                    }
                }

                let mut accumulator: u32 = 0;
                let mut accumulator_avg: u32 = 0;
                let mut accumulator_mass: u32 = 0;
                for particle in &particle_buffer {
                    if !(particle.fptr == -1.0 || particle.bptr == -1.0) {
                        accumulator = std::cmp::max(
                            accumulator,
                            ((particle.fptr as u32 - particle.bptr as u32) / 2) as u32,
                        );

                        accumulator_avg +=
                            ((particle.fptr as u32 - particle.bptr as u32) / 2) as u32;

                        accumulator_mass = std::cmp::max(particle.mass as u32, accumulator_mass);
                    }
                }

                accumulator_avg = accumulator_avg / particle_buffer.len() as u32;
                //accumulator_mass = accumulator_mass / particle_buffer.len() as u32;
                //println!("{:#?}", particle_buffer);
                for particle in particle_buffer {
                    println!("{} {} {} ", particle.fptr, particle.bptr, particle.mass);
                }
                println!("MAXIMUM DISTANCE CHECKED: {}", accumulator);
                println!("AVERAGE DISTANCE CHAECKED: {}", accumulator_avg);
                println!("MAX PARTICLES / CELL: {}", accumulator_mass);
            };

        let with_buffer_index =
            |result: result::Result<wgpu::util::DownloadBuffer, wgpu::BufferAsyncError>| {
                let buffer: &[u8] = &*result.unwrap();
                let mut index_buffer: Vec<i32> = Vec::new();

                for bytes in buffer.chunks_exact(std::mem::size_of::<i32>()) {
                    let bytes_fixed: *const [u8; std::mem::size_of::<i32>()] =
                        bytes.as_ptr() as *const [u8; std::mem::size_of::<i32>()];

                    unsafe {
                        let new_i32: i32 = mem::transmute(*bytes_fixed);
                        index_buffer.push(new_i32);
                    }
                }

                // println!("{:#?}", index_buffer);
            };

        println!("DEBUG PARTICLES ---------------");

        wgpu::util::DownloadBuffer::read_buffer(
            device,
            queue,
            &self.particle_buffers[1].slice(..),
            with_buffer,
        );

        println!("DEBUG INDEX BUFFER ----------");

        wgpu::util::DownloadBuffer::read_buffer(
            device,
            queue,
            &self.bucket_indeces_buffer.slice(..),
            with_buffer_index,
        )
    }
    pub fn render(
        &mut self,
        view: &wgpu::TextureView,
        resolve_view: Option<&TextureView>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        play: bool,
    ) {
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
            label: Some("My Render Pass"),
            color_attachments: &color_attachments,
            depth_stencil_attachment: None,
        };

        // get command encoder
        let mut command_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let mut cleanup_command_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        if play {
            //for i in 0..1 {
            let mut preprocessing_command_encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Preprocessing Command Encoder"),
                });

            {
                //preprocessing compute pass
                let mut ppass = preprocessing_command_encoder.begin_compute_pass(
                    &wgpu::ComputePassDescriptor {
                        label: Some("Preprocessing Pass"),
                    },
                );
                ppass.set_pipeline(&self.preprocessing_pipeline);
                ppass.set_bind_group(0, &self.preprocessing_bind_groups[0], &[]);
                ppass.dispatch_workgroups(self.work_group_count, 1, 1);
            }
            queue.submit(Some(preprocessing_command_encoder.finish()));
            //}
            // compute pass
            let mut cpass =
                command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &self.particle_bind_groups[0], &[]);
            cpass.dispatch_workgroups(self.work_group_count, 1, 1);

            // let mut clpass =
            //     cleanup_command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            //         label: Some("Cleanup Pass"),
            //     });
            // clpass.set_pipeline(&self.cleanup_pipeline);
            // clpass.set_bind_group(0, &self.cleanup_bind_groups[0], &[]);
            //
            // clpass.dispatch_workgroups(self.work_group_count, 1, 1);
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
            rpass.set_vertex_buffer(0, self.particle_buffers[0].slice(..));
            rpass.set_vertex_buffer(1, self.circle_buffer.slice(..));
            // the three instance-local vertices ????
            rpass.draw(
                0..((eden::CIRCLE_RES * 3) as u32),
                0..self.params.num_particles,
            );
        }

        // update frame count
        self.frame_num += 1;

        // done
        queue.submit(Some(command_encoder.finish()));
        // queue.submit(Some(cleanup_command_encoder.finish()));
    }
    fn post_processing(
        &mut self,
        view: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        //light effect
        //
        // let render_pass_descriptor = wgpu::RenderPassDescriptor {
        //    label: None,
        //    color_attachments: &color_attachments,
        //  depth_stencil_attachment: None,
        // };
    }
}
