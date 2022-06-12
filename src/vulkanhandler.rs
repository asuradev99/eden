use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
    dpi::LogicalSize,
};
use winit::event::{ElementState, VirtualKeyCode,};
use winit::error::OsError;
use erupt::utils::{
    surface,
    VulkanResult
};
// use erupt::vk::{
//     make_api_version, ApplicationInfoBuilder, Instance, InstanceCreateInfoBuilder, API_VERSION_1_0,
// };
use erupt::{
    vk,
    EntryLoader,
    InstanceLoader,
};
use std::ffi::{CString};
use std::error::Error;

macro_rules! cstr{
    ($str: expr) => {
        &CString::new($str)
            .unwrap()
    }
}


//init constants
const WIDTH: u32 = 800; 
const HEIGHT: u32 = 600;
const TITLE: &str = "Automata Simulator"; 

pub struct VulkanHandler {
    entry: EntryLoader,
    instance: InstanceLoader,
}
//private functions
impl VulkanHandler {
    
    //setup window
    

}

//public functions
impl VulkanHandler {
    pub fn init_window(event_loop: &EventLoop<()>) -> Result<Window, OsError> {
        let window = WindowBuilder::new()
            .with_title(TITLE)
            .with_inner_size(LogicalSize::new(WIDTH, HEIGHT))
            .build(event_loop)?;
        Ok(window)
    }
    //initialize entry, app, and instance
    pub fn new(window: &Window) -> Result<Self, Box<dyn Error>>{
        let entry = EntryLoader::new()?;
        let app_name = cstr!("Automata Simulator");
        let engine_name = cstr!("No Engine");
        let app_info = vk::ApplicationInfoBuilder::new()
            .api_version(vk::API_VERSION_1_1)
            .application_name(app_name)
            .application_version(vk::make_api_version(0, 1, 1, 0))
            .engine_name(engine_name)
            .engine_version(vk::API_VERSION_1_1); //change if probls
        
        let extensions = surface::enumerate_required_extensions(window).result()?;
       
        let create_info = vk::InstanceCreateInfoBuilder::new()
            .application_info(&app_info)
            .enabled_extension_names(&extensions);

        let instance = unsafe {
            InstanceLoader::new(&entry, &create_info)?
        };
       
        Ok(VulkanHandler {
            entry,
            instance, 
        })
    }

    //main run function with event loop (combined with main_loop)
    pub fn run(&self, event_loop: EventLoop<()>, window: Window) {

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
    
            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::KeyboardInput { input, .. } => {
                        if let (Some(VirtualKeyCode::Escape), ElementState::Released) =
                            (input.virtual_keycode, input.state)
                        {
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                    _ => (),
                },
                Event::MainEventsCleared => window.request_redraw(),
                Event::RedrawRequested(_) => {
                    // Required to drop VulkanApp
                   // self.draw_frame();
                }
                _ => (),
            }
        });
    }
}

//equivalent of cleanup() function
impl Drop for VulkanHandler {
    fn drop(&mut self) {
        unsafe { self.instance.destroy_instance(None)}
    }
}