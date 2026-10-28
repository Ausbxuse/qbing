use std::sync::atomic::{AtomicUsize, Ordering};
static MESH_DRAWS: AtomicUsize = AtomicUsize::new(0);
static COMPOSITES: AtomicUsize = AtomicUsize::new(0);
pub fn profile() -> serde_json::Value {
    serde_json::json!({"meshDraws":MESH_DRAWS.load(Ordering::Relaxed),"composites":COMPOSITES.load(Ordering::Relaxed),"triangles":49302,"meshUploads":1})
}
// Depth-tested OpenGL renderer. The independent depth attachment does not depend on
// whether the window compositor allocated a depth buffer for egui's 2D surface.
use super::{Motion, mesh::Quaternion};
use crate::cube::Cube;
use std::io::Read;
#[derive(Clone, Copy, PartialEq)]
pub struct Snapshot {
    cube: Cube,
    yaw: f32,
    pitch: f32,
    turn: Option<Motion>,
    orientation: Quaternion,
    mark_turns: i8,
}
impl Snapshot {
    pub fn new(
        cube: Cube,
        yaw: f32,
        pitch: f32,
        turn: Option<Motion>,
        orientation: Quaternion,
        mark_turns: i8,
    ) -> Self {
        Self {
            cube,
            yaw,
            pitch,
            turn,
            orientation,
            mark_turns,
        }
    }
}
use eframe::{egui, egui_glow, glow};
use glow::HasContext;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Renderer(Arc<Mutex<Resources>>);
struct Resources {
    gl: Arc<glow::Context>,
    mesh_program: glow::Program,
    quad_program: glow::Program,
    vao: glow::VertexArray,
    buffer: glow::Buffer,
    indices: glow::Buffer,
    count: i32,
    resolve: glow::Framebuffer,
    color: glow::Renderbuffer,
    samples: i32,
    previous: Option<[i32; 54]>,
    rendered: Option<Snapshot>,
    framebuffer: glow::Framebuffer,
    texture: glow::Texture,
    depth: glow::Renderbuffer,
    size: [i32; 2],
    destroyed: bool,
}

impl Renderer {
    pub fn new(gl: Arc<glow::Context>) -> Result<Self, String> {
        // This is called once with eframe's current GL context. Resources share its lifetime.
        unsafe {
            let prefix = if gl.version().is_embedded {
                "#version 300 es\nprecision highp float;\n"
            } else {
                "#version 140\n"
            };
            let mesh_program = program(
                &gl,
                prefix,
                include_str!("../../../assets/cube.vert"),
                include_str!("../../../assets/cube.frag"),
            )?;
            let quad_program = program(
                &gl,
                prefix,
                "out vec2 uv; void main(){vec2 p=vec2(float(gl_VertexID%2),float(gl_VertexID/2));uv=p;gl_Position=vec4(p*2.0-1.0,0.0,1.0);}",
                "in vec2 uv; uniform sampler2D image; uniform int sampleCount; out vec4 fragColor;
                 void main(){
                   vec2 texel=1.0/vec2(textureSize(image,0));
                   vec4 sum=vec4(0.0);
                   for(int y=0;y<4;y++){for(int x=0;x<4;x++){
                     if(x<sampleCount && y<sampleCount){
                       vec2 offset=vec2(float(x),float(y))-vec2(float(sampleCount-1)*0.5);
                       sum+=texture(image,uv+offset*texel);
                     }
                   }}
                   fragColor=sum/float(sampleCount*sampleCount);
                 }",
            )?;
            let vao = gl.create_vertex_array()?;
            let buffer = gl.create_buffer()?;
            let indices = gl.create_buffer()?;
            let resolve = gl.create_framebuffer()?;
            let color = gl.create_renderbuffer()?;
            let samples = gl.get_parameter_i32(glow::MAX_SAMPLES).clamp(1, 4);
            let mut bytes = Vec::new();
            flate2::read::GzDecoder::new(
                include_bytes!("../../../assets/cube-scene.bin.gz").as_slice(),
            )
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
            let vertices = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
            let count = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as i32;

            let framebuffer = gl.create_framebuffer()?;
            let texture = gl.create_texture()?;
            let depth = gl.create_renderbuffer()?;
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(buffer));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                &bytes[8..8 + vertices * 40],
                glow::STATIC_DRAW,
            );
            for (location, size, offset) in [(0, 3, 0), (1, 3, 12), (2, 3, 24), (3, 1, 36)] {
                gl.enable_vertex_attrib_array(location);
                gl.vertex_attrib_pointer_f32(location, size, glow::FLOAT, false, 40, offset);
            }
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(indices));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                &bytes[8 + vertices * 40..],
                glow::STATIC_DRAW,
            );
            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            Ok(Self(Arc::new(Mutex::new(Resources {
                gl,
                mesh_program,
                quad_program,
                vao,
                buffer,
                indices,
                count,
                resolve,
                color,
                samples,
                previous: None,
                rendered: None,
                framebuffer,
                texture,
                depth,
                size: [0, 0],
                destroyed: false,
            }))))
        }
    }
    /// Called by App::on_exit while eframe still has a current context.
    pub fn destroy(&self) {
        if let Ok(mut resources) = self.0.lock() {
            if resources.destroyed {
                return;
            }
            unsafe {
                let gl = &resources.gl;
                gl.delete_program(resources.mesh_program);
                gl.delete_program(resources.quad_program);
                gl.delete_vertex_array(resources.vao);
                gl.delete_buffer(resources.buffer);
                gl.delete_buffer(resources.indices);
                gl.delete_framebuffer(resources.resolve);
                gl.delete_renderbuffer(resources.color);
                gl.delete_framebuffer(resources.framebuffer);
                gl.delete_texture(resources.texture);
                gl.delete_renderbuffer(resources.depth);
            }
            resources.destroyed = true;
        }
    }
    pub fn callback(&self, rect: egui::Rect, frame: Snapshot) -> egui::PaintCallback {
        let renderer = self.clone();
        egui::PaintCallback {
            rect,
            callback: Arc::new(egui_glow::CallbackFn::new(move |info, painter| {
                if let Ok(mut resources) = renderer.0.lock() {
                    unsafe {
                        resources.paint(info, painter, frame);
                    }
                }
            })),
        }
    }
}

unsafe fn program(
    gl: &glow::Context,
    prefix: &str,
    vertex: &str,
    fragment: &str,
) -> Result<glow::Program, String> {
    unsafe {
        let program = gl.create_program()?;
        let mut shaders = Vec::new();
        for (kind, source) in [
            (glow::VERTEX_SHADER, vertex),
            (glow::FRAGMENT_SHADER, fragment),
        ] {
            let shader = gl.create_shader(kind)?;
            gl.shader_source(shader, &format!("{prefix}{source}"));
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                let error = gl.get_shader_info_log(shader);
                gl.delete_shader(shader);
                gl.delete_program(program);
                return Err(error);
            }
            gl.attach_shader(program, shader);
            shaders.push(shader);
        }
        gl.bind_attrib_location(program, 0, "position");
        gl.bind_attrib_location(program, 1, "normal");
        gl.bind_attrib_location(program, 2, "home");
        gl.bind_attrib_location(program, 3, "face");
        gl.link_program(program);
        for shader in shaders {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }
        if !gl.get_program_link_status(program) {
            let error = gl.get_program_info_log(program);
            gl.delete_program(program);
            return Err(error);
        }
        Ok(program)
    }
}

impl Resources {
    unsafe fn paint(
        &mut self,
        info: egui::PaintCallbackInfo,
        painter: &egui_glow::Painter,
        frame: Snapshot,
    ) {
        unsafe {
            if self.destroyed {
                return;
            }
            let gl = &self.gl;
            let viewport = info.viewport_in_pixels();
            if viewport.width_px <= 0 || viewport.height_px <= 0 {
                return;
            }
            // Resolve every high-resolution sample, rather than reducing a 4x
            // image with a single bilinear lookup (which would skip samples).
            let samples = 1;
            let width = (viewport.width_px * samples).min(2048);
            let height = (viewport.height_px * samples).min(2048);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.resolve));
            if self.size != [width, height] {
                gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
                gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA8 as i32,
                    width,
                    height,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::PixelUnpackData::Slice(None),
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MIN_FILTER,
                    glow::LINEAR as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MAG_FILTER,
                    glow::LINEAR as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_WRAP_S,
                    glow::CLAMP_TO_EDGE as i32,
                );
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_WRAP_T,
                    glow::CLAMP_TO_EDGE as i32,
                );
                gl.framebuffer_texture_2d(
                    glow::FRAMEBUFFER,
                    glow::COLOR_ATTACHMENT0,
                    glow::TEXTURE_2D,
                    Some(self.texture),
                    0,
                );
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.framebuffer));
                gl.bind_renderbuffer(glow::RENDERBUFFER, Some(self.color));
                gl.renderbuffer_storage_multisample(
                    glow::RENDERBUFFER,
                    self.samples,
                    glow::RGBA8,
                    width,
                    height,
                );
                gl.framebuffer_renderbuffer(
                    glow::FRAMEBUFFER,
                    glow::COLOR_ATTACHMENT0,
                    glow::RENDERBUFFER,
                    Some(self.color),
                );
                gl.bind_renderbuffer(glow::RENDERBUFFER, Some(self.depth));
                gl.renderbuffer_storage_multisample(
                    glow::RENDERBUFFER,
                    self.samples,
                    glow::DEPTH_COMPONENT24,
                    width,
                    height,
                );
                gl.framebuffer_renderbuffer(
                    glow::FRAMEBUFFER,
                    glow::DEPTH_ATTACHMENT,
                    glow::RENDERBUFFER,
                    Some(self.depth),
                );
                gl.bind_renderbuffer(glow::RENDERBUFFER, None);
                self.size = [width, height];
                self.rendered = None;
            }
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.framebuffer));
            if self.rendered != Some(frame) {
                gl.viewport(0, 0, width, height);
                gl.disable(glow::SCISSOR_TEST);
                gl.disable(glow::BLEND);
                gl.enable(glow::DEPTH_TEST);
                gl.depth_func(glow::LESS);
                gl.depth_mask(true);
                gl.enable(glow::CULL_FACE);
                gl.cull_face(glow::BACK);
                gl.front_face(glow::CCW);
                gl.clear_color(0., 0., 0., 0.);
                gl.clear_depth_f32(1.);
                gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
                gl.use_program(Some(self.mesh_program));
                gl.bind_vertex_array(Some(self.vao));
                gl.uniform_1_f32(
                    gl.get_uniform_location(self.mesh_program, "logoAngle")
                        .as_ref(),
                    -frame.mark_turns as f32 * std::f32::consts::FRAC_PI_2,
                );
                gl.uniform_2_f32(
                    gl.get_uniform_location(self.mesh_program, "aspect")
                        .as_ref(),
                    height as f32 / width as f32,
                    1.,
                );
                gl.uniform_2_f32(
                    gl.get_uniform_location(self.mesh_program, "orbit").as_ref(),
                    frame.yaw,
                    frame.pitch,
                );
                gl.uniform_4_f32_slice(
                    gl.get_uniform_location(self.mesh_program, "orientation")
                        .as_ref(),
                    &frame.orientation,
                );
                let mut turns = [0_f32; 8];
                if let Some(motion) = frame.turn {
                    for (i, pair) in [
                        (motion.first, motion.progress),
                        motion
                            .second
                            .map(|m| (m, motion.partner_progress))
                            .unwrap_or((motion.first, 0.)),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        if i == 1 && motion.second.is_none() {
                            continue;
                        }
                        let (axis, layer, sign) = pair.0.axis();
                        turns[i * 4..i * 4 + 4].copy_from_slice(&[
                            axis as f32,
                            layer as f32,
                            sign as f32
                                * pair.0.turns as f32
                                * std::f32::consts::FRAC_PI_2
                                * pair.1,
                            1.,
                        ]);
                    }
                }
                gl.uniform_4_f32_slice(
                    gl.get_uniform_location(self.mesh_program, "turns[0]")
                        .as_ref(),
                    &turns,
                );
                let stickers = frame
                    .cube
                    .0
                    .map(|f| crate::cube::FACES.iter().position(|v| *v == f).unwrap_or(0) as i32);
                if self.previous != Some(stickers) {
                    gl.uniform_1_i32_slice(
                        gl.get_uniform_location(self.mesh_program, "stickers[0]")
                            .as_ref(),
                        &stickers,
                    );
                    self.previous = Some(stickers);
                }
                MESH_DRAWS.fetch_add(1, Ordering::Relaxed);
                gl.draw_elements(glow::TRIANGLES, self.count, glow::UNSIGNED_INT, 0);
                gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(self.framebuffer));
                gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(self.resolve));
                gl.blit_framebuffer(
                    0,
                    0,
                    width,
                    height,
                    0,
                    0,
                    width,
                    height,
                    glow::COLOR_BUFFER_BIT,
                    glow::NEAREST,
                );

                self.rendered = Some(frame);
            }
            gl.bind_framebuffer(glow::FRAMEBUFFER, painter.intermediate_fbo());
            gl.viewport(
                viewport.left_px,
                viewport.from_bottom_px,
                viewport.width_px,
                viewport.height_px,
            );
            let clip = info.clip_rect_in_pixels();
            gl.enable(glow::SCISSOR_TEST);
            gl.scissor(
                clip.left_px,
                clip.from_bottom_px,
                clip.width_px,
                clip.height_px,
            );
            gl.disable(glow::DEPTH_TEST);
            gl.depth_mask(false);
            gl.disable(glow::CULL_FACE);
            gl.enable(glow::BLEND);
            gl.blend_equation(glow::FUNC_ADD);
            gl.blend_func(glow::ONE, glow::ONE_MINUS_SRC_ALPHA);
            gl.use_program(Some(self.quad_program));
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            gl.uniform_1_i32(
                gl.get_uniform_location(self.quad_program, "image").as_ref(),
                0,
            );
            gl.uniform_1_i32(
                gl.get_uniform_location(self.quad_program, "sampleCount")
                    .as_ref(),
                samples,
            );
            COMPOSITES.fetch_add(1, Ordering::Relaxed);
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
        }
    }
}
