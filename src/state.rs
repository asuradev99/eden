
//use nanorand::{Rng, WyRand};
use rand::prelude::*;
use std::{borrow::Cow, mem};

use wgpu::util::DeviceExt;



#[derive(Debug, Copy, Clone)]
pub struct Camera {
    pub x: f32, 
    pub y: f32,
    pub zoom: f32, 
}

impl Camera {
    pub fn new() -> Self {
        Camera {
            x: 0.0, 
            y: 0.0,
            zoom: 1.0, 
        }
    }

    pub fn to_slice(&self) -> [f32; 3] {
        [
            self.x, 
            self.y, 
            self.zoom
        ]
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Params {
    pub g: f32, 
    pub dt: f32,
    pub num_particles: u32,
}

impl Params {
    pub fn new() -> Self {
        Params {
            g: 0.01,
            dt: 0.1, 
            num_particles: 1000,
        }
    }
    pub fn to_slice(&self) -> [f32; 2] {
        [
            self.g, 
            self.dt, 
        ]
    }
}

// const PARAMS: [f32; 2] = [
//     0.001, //dt
//     0.01//Gravitational constant
// ];


/// Example struct holds references to wgpu resources and frame persistent data
pub struct State {
    particle_bind_groups: Vec<wgpu::BindGroup>,
    particle_buffers: Vec<wgpu::Buffer>,
    compute_pipeline: wgpu::ComputePipeline,
    render_pipeline: wgpu::RenderPipeline,
    work_group_count: u32,
    frame_num: usize,
    pub camera: Camera,
    pub camera_uniform_buffer: wgpu::Buffer, 
    pub params: Params,
    camera_bind_group: wgpu::BindGroup,

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
        params: Params,
        config: &wgpu::SurfaceConfiguration,
        _adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) -> Self {

        //create parameters 
        let params = params; 
        let params_slice = params.to_slice();

        //initialize compute shader module
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shaders/compute.wgsl"))),
        });

        //initialize vertex and fragment shaders
        let draw_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shaders/draw.wgsl"))),
        });

        //set up uniform buffer to store global parameters
        let sim_param_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Parameter Buffer"),
            contents: bytemuck::cast_slice(&(params_slice)),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        //set up camera buffer
        let camera = Camera::new();
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
                        )
                    },
                    count: None,
                },

                //input / source buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE, 
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {read_only: true},
                        has_dynamic_offset: false, 
                        min_binding_size: wgpu::BufferSize::new((params.num_particles * 16) as _), //CHANGE SIZE IF ISSUES
                    },
                    count: None,
                },

                //output / destination buffer
                wgpu::BindGroupLayoutEntry {
                    binding: 2, 
                    visibility: wgpu::ShaderStages::COMPUTE, 
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {read_only: false},
                        has_dynamic_offset: false, 
                        min_binding_size: wgpu::BufferSize::new((params.num_particles * 16) as _),
                    },
                    count: None,
                },
                ], 
                label: None
            });
        //compute pipeline layout = 
        let compute_pipeline_layout = 
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            });

        //camera bind group layout
        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("camera_bind_group_layout"),
        });

        //render pipeline layout
        let render_pipeline_layout = 
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render"),
                bind_group_layouts: &[ &camera_bind_group_layout], 
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
                        array_stride: 4 * 4, 
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
                    },
                    
                ],

            },
            fragment: Some(wgpu::FragmentState {
                module: &draw_shader,
                entry_point: "main_fs",
                targets: &[Some(config.format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::PointList, // change to PointLIst
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });


        // create compute pipeline

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "main",
        });

        // // buffer for the three 2d triangle vertices of each instance
        // let vertex_buffer_data = [-0.01f32, -0.02, 0.01, -0.02, 0.00, 0.02];
        // let vertices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //     label: Some("Vertex Buffer"),
        //     contents: bytemuck::bytes_of(&vertex_buffer_data),
        //     usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        // });


        // buffer for all particles data of type [(posx,posy,velx,vely),...]
        let mut initial_particle_data = vec![0.0f32; (4 * (params.num_particles)) as usize];
        
        //generate random pos and vel
        let mut rng = rand::thread_rng();
        let mut unif = || rng.gen::<f32>() * 2f32 - 1f32; // Generate a num (-1, 1)
        for particle_instance_chunk in initial_particle_data.chunks_mut(4) {
            particle_instance_chunk[0] = unif(); // posx
            particle_instance_chunk[1] = unif(); // posy
           
        };

        //println!("{:?}", initial_particle_data);
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
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_uniform_buffer.as_entire_binding(),
                }
            ],
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
                ],
                label: None,
            }));
        }
        
        
        // calculates number of work groups from PARTICLES_PER_GROUP constant
        let work_group_count = u32::min(params.num_particles, 65535);

        // returns Example struct and No encoder commands

        State {
            particle_bind_groups,
            particle_buffers,
            compute_pipeline,
            render_pipeline,
            work_group_count,
            frame_num: 0,
            camera,
            camera_uniform_buffer, 
            params, 
            camera_bind_group
        }

    }

    /// update is called for any WindowEvent not handled by the framework
    pub  fn update(&mut self, _event: winit::event::WindowEvent) {
        //empty
    }

    /// resize is called on WindowEvent::Resized events
    pub fn resize(
        &mut self,
        _sc_desc: &wgpu::SurfaceConfiguration,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) {
        //empty
    }
    pub fn render(
        &mut self,
        view: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        // create render pass descriptor and its color attachments
        let color_attachments = [Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
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

        //compute pass
        {
            // compute pass
            let mut cpass =
                command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
            cpass.set_pipeline(&self.compute_pipeline);
            cpass.set_bind_group(0, &self.particle_bind_groups[self.frame_num % 2], &[]);
            cpass.dispatch_workgroups(self.work_group_count, 1, 1);
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
            // the three instance-local vertices ????
            rpass.draw(0..1, 0..self.params.num_particles );
        }

        // update frame count
        self.frame_num += 1;

        // done
        queue.submit(Some(command_encoder.finish()));
    }
}
