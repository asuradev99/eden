pub mod vulkanhandler;
use winit::event_loop::EventLoop;

use vulkanhandler::VulkanHandler;

fn main () {
   let event_loop = EventLoop::new();
   let window = VulkanHandler::init_window(&event_loop).unwrap();
   let vk_handler = VulkanHandler::new(&window).unwrap();
   vk_handler.run(event_loop, window);
}