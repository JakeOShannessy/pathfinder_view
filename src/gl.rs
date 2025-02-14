
use pathfinder_gl::{GLDevice, GLVersion};
use pathfinder_renderer::{
    concurrent::{
        rayon::RayonExecutor,
        scene_proxy::SceneProxy,
        executor::SequentialExecutor,
    },
    gpu::{
        options::{DestFramebuffer, RendererOptions, RendererMode, RendererLevel},
        renderer::Renderer
    },
    scene::Scene,
    options::{BuildOptions}
};
use pathfinder_geometry::{
    vector::{Vector2F, Vector2I},
    rect::RectF
};

use glutin::{GlRequest, Api, WindowedContext, PossiblyCurrent};
use winit::{
    event_loop::EventLoop,
    window::{WindowBuilder, Window},
    dpi::{PhysicalSize},
};
use gl;
use crate::Config;
use crate::util::round_v_to_16;

pub struct GlWindow {
    windowed_context: WindowedContext<PossiblyCurrent>,
    proxy: SceneProxy,
    renderer: Renderer<GLDevice>,
    framebuffer_size: Vector2I,
    window_size: Vector2F,
}
impl GlWindow {
    pub fn new<T>(event_loop: &EventLoop<T>, title: String, window_size: Vector2F, config: &Config) -> Self {
        let window_builder = WindowBuilder::new()
            .with_title(title)
            .with_decorations(config.borders)
            .with_inner_size(PhysicalSize::new(window_size.x() as f64, window_size.y() as f64))
            .with_transparent(config.transparent);

        let (glutin_gl_version, renderer_gl_version) = match config.render_level {
            RendererLevel::D3D9 => ((3, 0), GLVersion::GLES3),
            RendererLevel::D3D11 => ((4, 3), GLVersion::GL4),
        };
        let windowed_context = glutin::ContextBuilder::new()
            .with_gl(GlRequest::Specific(Api::OpenGl, glutin_gl_version))
            .build_windowed(window_builder, &event_loop)
            .unwrap();
        
        let windowed_context = unsafe {
            windowed_context.make_current().unwrap()
        };

        gl::load_with(|ptr| windowed_context.get_proc_address(ptr));
        
        let dpi = windowed_context.window().scale_factor() as f32;
        let proxy = match config.threads {
            true => SceneProxy::new(config.render_level, RayonExecutor),
            false => SceneProxy::new(config.render_level, SequentialExecutor)
        };
        let framebuffer_size = (window_size * dpi).to_i32();
        // Create a Pathfinder renderer.
        let render_mode = RendererMode { level: config.render_level };
        let render_options = RendererOptions {
            dest:  DestFramebuffer::full_window(framebuffer_size),
            background_color: Some(config.background),
            show_debug_ui: false,
        };


        let renderer = Renderer::new(GLDevice::new(renderer_gl_version, 0),
            &*config.resource_loader,
            render_mode,
            render_options,
        );

        GlWindow {
            windowed_context,
            proxy,
            renderer,
            framebuffer_size,
            window_size,
        }
    }
    pub fn render(&mut self, mut scene: Scene, options: BuildOptions) {
        scene.set_view_box(RectF::new(Vector2F::default(), self.framebuffer_size.to_f32()));
        self.proxy.replace_scene(scene);

        self.proxy.build_and_render(&mut self.renderer, options);
        self.windowed_context.swap_buffers().unwrap();
    }
    
    pub fn resize(&mut self, size: Vector2F) {
        if size != self.window_size {
            let window = self.windowed_context.window();
            window.set_inner_size(PhysicalSize::new(size.x() as u32, size.y() as u32));
            window.request_redraw();
            self.window_size = size;
        }
    }
    // size changed, update GL context
    pub fn resized(&mut self, size: Vector2F) {
        // pathfinder does not like scene sizes that are now a multiple of the tile size (16).
        let new_framebuffer_size = round_v_to_16(size.to_i32());
        if new_framebuffer_size != self.framebuffer_size {
            self.framebuffer_size = new_framebuffer_size;
            self.windowed_context.resize(PhysicalSize::new(self.framebuffer_size.x() as u32, self.framebuffer_size.y() as u32));
            self.renderer.options_mut().dest = DestFramebuffer::full_window(new_framebuffer_size);
        }
    }
    pub fn scale_factor(&self) -> f32 {
        self.windowed_context.window().scale_factor() as f32
    }
    pub fn request_redraw(&self) {
        self.windowed_context.window().request_redraw();
    }
    pub fn framebuffer_size(&self) -> Vector2I {
        self.framebuffer_size
    }
    pub fn window(&self) -> &Window {
        self.windowed_context.window()
    }
}
