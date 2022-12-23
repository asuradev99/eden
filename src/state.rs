//use nanorand::{Rng, WyRand};
use rand::prelude::*;
use std::{borrow::Cow, mem};

use wgpu::util::DeviceExt;

#[derive(Debug, Copy, Clone)]
pub struct Camera {
    pub x: f32,
    pub y: f32,
    pub zoom: f32,
    pub aspect_ratio: f32,
}

const CIRCLE_RES: u32 = 128;

impl Camera {
    pub fn new(zoom: f32, aspect_ratio: f32) -> Self {
        Camera {
            x: 0.0,
            y: 0.0,
            zoom: zoom,
            aspect_ratio: aspect_ratio,
        }
    }

    pub fn to_slice(&self) -> [f32; 4] {
        [self.x, self.y, self.zoom, self.aspect_ratio]
    }
}

#[derive(Debug, Clone)]
pub struct Params {
    pub g: f32,
    pub dt: f32,
    pub num_particles: u32,
    pub world_size: f32,
    pub shader_buffer: String,
}


impl Params {
    pub fn new() -> Self {
        Params {
            g: 0.1,
            dt: 0.01,
            num_particles: 100,
            shader_buffer: crate::DEFAULT_COMPUTE_SHADER.to_string(),
            world_size: 100.0,
        }
    }
    pub fn to_slice(&self) -> [f32; 2] {
        [self.g, self.dt]
    }
}

// const PARAMS: [f32; 2] = [
//     0.001, //dt
//     0.01//Gravitational constant
// ];

/// Example struct holds references to wgpu resources and frame persistent data
/// 
struct Particle {
    pos: (f32, f32),
    vel: (f32, f32),
    //mass: f32
}

impl Particle {
    pub fn to_slice(&self) -> [f32; 4] {
        [self.pos.0, self.pos.1, self.vel.0, self.vel.1] //self.mass]
    }
    pub fn new_random(params: &Params) -> Self {
        let mut rng = rand::thread_rng();
        let mut unif = || (rng.gen::<f32>() * 2f32 - 1f32) * params.world_size;

        let mut rng = rand::thread_rng();
        let mut unif_mass = || (rng.gen::<f32>() * 2f32 - 1f32) * 10.0;

        Self {
            pos: (unif(), unif()),
            vel: (0.0, 0.0),
            //mass: 1.0
        }
    }

}

fn generate_circle(radius: f32) -> [f32; (CIRCLE_RES * 2)  as usize ] {
    use std::f64::consts::PI;
    use std::convert::TryInto;
    let mut coords = Vec::<f32>::new();
    for i in 0..CIRCLE_RES {
        coords.push(radius * ((2.0 * PI * i as f64 / CIRCLE_RES as f64) as f32).cos());
        coords.push(radius * ((2.0 * PI * i as f64 / CIRCLE_RES as f64) as f32).sin());
    }
    coords.try_into().unwrap_or_else(|v: Vec<f32>| panic!("Expected a Vec of length {} but it was {}", CIRCLE_RES, v.len()))
}

pub struct State {
    particle_bind_groups: Vec<wgpu::BindGroup>,
    particle_buffers: Vec<wgpu::Buffer>,
    circle_buffer: wgpu::Buffer,
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

        println!("{}", mem::size_of::<Particle>());
        //create parameters
        let params = params;
        let params_slice = params.to_slice();

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
            contents: bytemuck::cast_slice(&(params_slice)),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        //set up camera buffer
       // let camera = Camera::new(1.0 / (params.world_size * 1.5));
       let aspect_ratio:f32 = config.width as f32 / config.height as f32;
       println!("{} / {} = {}", config.width, config.height, aspect_ratio);
       let camera = Camera::new(1.0 / params.world_size, aspect_ratio);
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

 
        //buffer for particle circle coordinates

      //  let circle_buffer_data = [-0.01f32, -0.02, 0.01, -0.02, 0.00, 0.02];
        let circle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::bytes_of(&generate_circle( 0.5)),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // buffer for all particles data of type [(posx,posy,velx,vely),...]
        let mut initial_particle_data = vec![0.0f32; (6 * (params.num_particles)) as usize];

        //generate random pos and vel
        let mut rng = rand::thread_rng();
        let mut unif = || (rng.gen::<f32>() * 2f32 - 1f32) * params.world_size;
        let mut  bigsmall: u32 = 1; // Generate a num (-1, 1)
        for particle_instance_chunk in initial_particle_data.chunks_mut(6 as usize) {
            particle_instance_chunk[0] = unif(); // posx
            particle_instance_chunk[1] = unif();
            particle_instance_chunk[4] = unif() * ((bigsmall % 2) as f32) * 100.0 + 1.0 ; 
            bigsmall += 1;// posy
        }

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
    pub fn render(&mut self, view: &wgpu::TextureView, device: &wgpu::Device, queue: &wgpu::Queue) {
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
            rpass.set_vertex_buffer(1, self.circle_buffer.slice(..));
            // the three instance-local vertices ????
            rpass.draw(0..((self.circle_buffer.size() / 8) as u32), 0..self.params.num_particles);
        }

        // update frame count
        self.frame_num += 1;

        // done
        queue.submit(Some(command_encoder.finish()));
    }
}
