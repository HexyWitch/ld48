use std::rc::Rc;

use glow::HasContext;
use thiserror::Error;
use zerocopy::AsBytes;

type VertexArrayId = <glow::Context as glow::HasContext>::VertexArray;
type BufferId = <glow::Context as glow::HasContext>::Buffer;
type UniformLocationId = <glow::Context as glow::HasContext>::UniformLocation;
type ProgramId = <glow::Context as glow::HasContext>::Program;
type ShaderId = <glow::Context as glow::HasContext>::Shader;
type TextureId = <glow::Context as glow::HasContext>::Texture;

pub struct Shader(Rc<ShaderId>);
pub struct Texture {
    context: Rc<glow::Context>,
    texture_id: Rc<TextureId>,
    format: TextureFormat,
}
pub struct VertexBuffer {
    context: Rc<glow::Context>,
    vertex_array: Rc<VertexArrayId>,
    buffer: Rc<BufferId>,
    len: usize,
}

pub struct Context {
    context: Rc<glow::Context>,
    shaders: Vec<Rc<ShaderId>>,
    programs: Vec<Rc<ProgramId>>,
    vertex_arrays: Vec<Rc<VertexArrayId>>,
    buffers: Vec<Rc<BufferId>>,
    textures: Vec<Rc<TextureId>>,
}

#[derive(Debug, Error)]
#[error("OpenGL error: {0}")]
pub struct GLError(String);

impl Context {
    pub fn from_glow_context(context: glow::Context) -> Context {
        Context {
            context: Rc::new(context),
            shaders: Vec::new(),
            programs: Vec::new(),
            vertex_arrays: Vec::new(),
            buffers: Vec::new(),
            textures: Vec::new(),
        }
    }

    pub unsafe fn create_shader(
        &mut self,
        shader_type: ShaderType,
        src: &str,
    ) -> Result<Shader, GLError> {
        let shader_id = self
            .context
            .create_shader(shader_type as u32)
            .map_err(GLError)?;
        self.context.shader_source(shader_id, src);
        self.context.compile_shader(shader_id);
        if !self.context.get_shader_compile_status(shader_id) {
            Err(GLError(self.context.get_shader_info_log(shader_id)))
        } else {
            let shader = Shader(Rc::new(shader_id));
            self.shaders.push(shader.0.clone());
            Ok(shader)
        }
    }

    pub unsafe fn create_program(&mut self, desc: &ProgramDescriptor) -> Result<Program, GLError> {
        let program_id = self.context.create_program().map_err(GLError)?;
        self.context
            .attach_shader(program_id, *desc.vertex_shader.0);
        self.context
            .attach_shader(program_id, *desc.fragment_shader.0);
        self.context.link_program(program_id);
        if !self.context.get_program_link_status(program_id) {
            return Err(GLError(self.context.get_program_info_log(program_id)));
        }

        let mut set_uniforms = Vec::new();
        for entry in desc.uniforms {
            let location = self
                .context
                .get_uniform_location(program_id, entry.name)
                .ok_or_else(|| {
                    GLError(format!("could not get location for uniform {}", entry.name))
                })?;
            set_uniforms.push((location, None));
        }

        let vertex_format = VertexFormatInner {
            stride: desc.vertex_format.stride as i32,
            attributes: desc
                .vertex_format
                .attributes
                .iter()
                .map(|attr_desc| {
                    let location = self
                        .context
                        .get_attrib_location(program_id, attr_desc.name)
                        .ok_or_else(|| {
                            GLError(format!(
                                "could not get location of attribute {}",
                                attr_desc.name
                            ))
                        })?;
                    let attribute = VertexAttributeInner {
                        ty: attr_desc.ty,
                        size: attr_desc.size,
                        offset: attr_desc.offset,
                    };
                    Ok((location, attribute))
                })
                .collect::<Result<Vec<_>, GLError>>()?,
        };

        let program_id = Rc::new(program_id);
        self.programs.push(program_id.clone());
        Ok(Program {
            context: self.context.clone(),
            program_id: program_id,
            vertex_shader: desc.vertex_shader.0.clone(),
            fragment_shader: desc.fragment_shader.0.clone(),
            uniform_entry_types: desc.uniforms.iter().map(|e| e.ty).collect(),
            set_uniforms,
            vertex_format,
        })
    }

    pub unsafe fn create_vertex_buffer(&mut self) -> Result<VertexBuffer, GLError> {
        let vertex_array_id = Rc::new(self.context.create_vertex_array().map_err(GLError)?);
        self.vertex_arrays.push(vertex_array_id.clone());
        let buffer_id = Rc::new(self.context.create_buffer().map_err(GLError)?);
        self.buffers.push(buffer_id.clone());

        Ok(VertexBuffer {
            context: self.context.clone(),
            vertex_array: vertex_array_id,
            buffer: buffer_id,
            len: 0,
        })
    }

    pub unsafe fn create_texture(
        &mut self,
        format: TextureFormat,
        width: u32,
        height: u32,
    ) -> Result<Texture, GLError> {
        let texture_id = self.context.create_texture().map_err(GLError)?;
        self.context
            .bind_texture(glow::TEXTURE_2D, Some(texture_id));
        self.context.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::NEAREST as i32,
        );
        self.context.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::NEAREST as i32,
        );
        self.context.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_S,
            glow::CLAMP_TO_EDGE as i32,
        );
        self.context.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_WRAP_T,
            glow::CLAMP_TO_EDGE as i32,
        );

        self.context.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            match format {
                TextureFormat::RFloat | TextureFormat::RInt => glow::RED,
                TextureFormat::RGFloat | TextureFormat::RGInt => glow::RG,
                TextureFormat::RGBFloat | TextureFormat::RGBInt => glow::RGB,
                TextureFormat::BGRFloat | TextureFormat::BGRInt => glow::BGR,
                TextureFormat::RGBAFloat | TextureFormat::RGBAInt => glow::RGBA,
                TextureFormat::BGRAFloat | TextureFormat::BGRAInt => glow::BGRA,
            } as i32,
            width as i32,
            height as i32,
            0,
            match format {
                TextureFormat::RFloat => glow::RED,
                TextureFormat::RGFloat => glow::RG,
                TextureFormat::RGBFloat => glow::RGB,
                TextureFormat::BGRFloat => glow::BGR,
                TextureFormat::RGBAFloat => glow::RGBA,
                TextureFormat::BGRAFloat => glow::BGRA,
                TextureFormat::RInt => glow::RED_INTEGER,
                TextureFormat::RGInt => glow::RG_INTEGER,
                TextureFormat::RGBInt => glow::RGB_INTEGER,
                TextureFormat::BGRInt => glow::BGR_INTEGER,
                TextureFormat::RGBAInt => glow::RGBA_INTEGER,
                TextureFormat::BGRAInt => glow::BGRA_INTEGER,
            },
            glow::UNSIGNED_BYTE,
            None,
        );

        let texture_id = Rc::new(texture_id);
        self.textures.push(texture_id.clone());
        Ok(Texture {
            context: self.context.clone(),
            texture_id,
            format,
        })
    }

    pub unsafe fn maintain(&mut self) {
        for i in (0..self.programs.len()).rev() {
            if Rc::strong_count(&self.programs[i]) == 1 {
                let program = self.programs.swap_remove(i);
                self.context.delete_program(*program);
            }
        }
        for i in (0..self.shaders.len()).rev() {
            if Rc::strong_count(&self.shaders[i]) == 1 {
                let shader = self.shaders.swap_remove(i);
                self.context.delete_shader(*shader);
            }
        }
        for i in (0..self.vertex_arrays.len()).rev() {
            if Rc::strong_count(&self.vertex_arrays[i]) == 1 {
                let vertex_array = self.vertex_arrays.swap_remove(i);
                self.context.delete_vertex_array(*vertex_array);
            }
        }
        for i in (0..self.buffers.len()).rev() {
            if Rc::strong_count(&self.buffers[i]) == 1 {
                let buffer = self.buffers.swap_remove(i);
                self.context.delete_buffer(*buffer);
            }
        }
        for i in (0..self.textures.len()).rev() {
            if Rc::strong_count(&self.textures[i]) == 1 {
                let texture = self.textures.swap_remove(i);
                self.context.delete_texture(*texture);
            }
        }
    }

    pub unsafe fn clear(&mut self, color: [f32; 4]) {
        self.context
            .clear_color(color[0], color[1], color[2], color[3]);
        self.context.clear(glow::COLOR_BUFFER_BIT);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TextureFormat {
    RFloat,
    RInt,
    RGFloat,
    RGInt,
    RGBFloat,
    RGBInt,
    BGRFloat,
    BGRInt,
    RGBAFloat,
    RGBAInt,
    BGRAFloat,
    BGRAInt,
}

impl VertexBuffer {
    pub unsafe fn write<V: AsBytes>(&mut self, vertices: &[V]) {
        self.len = vertices.len();
        self.context.bind_vertex_array(Some(*self.vertex_array));
        self.context
            .bind_buffer(glow::ARRAY_BUFFER, Some(*self.buffer));
        self.context.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            vertices.as_bytes(),
            glow::STATIC_DRAW,
        );
    }
}

impl Texture {
    pub unsafe fn write(&mut self, x: u32, y: u32, width: u32, height: u32, data: &[u8]) {
        self.context
            .bind_texture(glow::TEXTURE_2D, Some(*self.texture_id));
        self.context.tex_sub_image_2d_u8_slice(
            glow::TEXTURE_2D,
            0,
            x as i32,
            y as i32,
            width as i32,
            height as i32,
            match self.format {
                TextureFormat::RFloat | TextureFormat::RInt => glow::RED,
                TextureFormat::RGFloat | TextureFormat::RGInt => glow::RG,
                TextureFormat::RGBFloat | TextureFormat::RGBInt => glow::RGB,
                TextureFormat::BGRFloat | TextureFormat::BGRInt => glow::BGR,
                TextureFormat::RGBAFloat | TextureFormat::RGBAInt => glow::RGBA,
                TextureFormat::BGRAFloat | TextureFormat::BGRAInt => glow::BGRA,
            },
            glow::UNSIGNED_BYTE,
            Some(data),
        );
    }
}

#[repr(u32)]
pub enum ShaderType {
    Vertex = glow::VERTEX_SHADER,
    Fragment = glow::FRAGMENT_SHADER,
}

struct VertexFormatInner {
    stride: i32,
    attributes: Vec<(VertexAttributeLocation, VertexAttributeInner)>,
}

struct VertexAttributeInner {
    pub ty: VertexAttributeType,
    pub size: u32,
    pub offset: u32,
}

pub struct Program {
    context: Rc<glow::Context>,
    program_id: Rc<ProgramId>,
    vertex_shader: Rc<ShaderId>,
    fragment_shader: Rc<ShaderId>,
    uniform_entry_types: Vec<UniformType>,
    set_uniforms: Vec<(UniformLocationId, Option<SetUniformValue>)>,
    vertex_format: VertexFormatInner,
}

impl Program {
    pub fn set_uniform(&mut self, index: usize, value: Uniform<'_>) -> Result<(), GLError> {
        if index > self.set_uniforms.len() {
            return Err(GLError(format!("Uniform index {} is out of range", index)));
        }
        if value.uniform_type() != self.uniform_entry_types[index] {
            return Err(GLError(format!(
                "Wrong uniform type. Expected: {:?} Got uniform of type: {:?}",
                self.uniform_entry_types[index],
                value.uniform_type()
            )));
        }
        self.set_uniforms[index].1 = match value {
            Uniform::Texture(texture) => Some(SetUniformValue::Texture(texture.texture_id.clone())),
            Uniform::Int(x) => Some(SetUniformValue::Int(x)),
            Uniform::Int2(x, y) => Some(SetUniformValue::Int2(x, y)),
            Uniform::Int3(x, y, z) => Some(SetUniformValue::Int3(x, y, z)),
            Uniform::Int4(x, y, z, w) => Some(SetUniformValue::Int4(x, y, z, w)),
            Uniform::Float(x) => Some(SetUniformValue::Float(x)),
            Uniform::Float2(x, y) => Some(SetUniformValue::Float2(x, y)),
            Uniform::Float3(x, y, z) => Some(SetUniformValue::Float3(x, y, z)),
            Uniform::Float4(x, y, z, w) => Some(SetUniformValue::Float4(x, y, z, w)),
            Uniform::Mat2(m) => Some(SetUniformValue::Mat2(m)),
            Uniform::Mat3(m) => Some(SetUniformValue::Mat3(m)),
            Uniform::Mat4(m) => Some(SetUniformValue::Mat4(m)),
        };

        Ok(())
    }

    pub unsafe fn render_vertices(&self, vertex_buffer: &VertexBuffer) -> Result<(), GLError> {
        self.context
            .blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
        self.context.enable(glow::BLEND);

        self.context
            .bind_vertex_array(Some(*vertex_buffer.vertex_array));
        self.context
            .bind_buffer(glow::ARRAY_BUFFER, Some(*vertex_buffer.buffer));

        self.context.use_program(Some(*self.program_id));

        let mut texture_index = 0;
        for (i, (location, uniform_value)) in self.set_uniforms.iter().enumerate() {
            if uniform_value.is_none() {
                return Err(GLError(format!("uniform {} is not set", i)));
            }
            match uniform_value.as_ref().unwrap() {
                SetUniformValue::Texture(texture) => {
                    self.context.active_texture(glow::TEXTURE0 + texture_index);
                    self.context.bind_texture(glow::TEXTURE_2D, Some(**texture));
                    self.context
                        .uniform_1_i32(Some(location.clone()), texture_index as i32);
                    texture_index += 1;
                }
                SetUniformValue::Int(x) => {
                    self.context.uniform_1_i32(Some(location.clone()), *x);
                }
                SetUniformValue::Int2(x, y) => {
                    self.context.uniform_2_i32(Some(location.clone()), *x, *y);
                }
                SetUniformValue::Int3(x, y, z) => {
                    self.context
                        .uniform_3_i32(Some(location.clone()), *x, *y, *z);
                }
                SetUniformValue::Int4(x, y, z, w) => {
                    self.context
                        .uniform_4_i32(Some(location.clone()), *x, *y, *z, *w);
                }
                SetUniformValue::Float(x) => {
                    self.context.uniform_1_f32(Some(location.clone()), *x);
                }
                SetUniformValue::Float2(x, y) => {
                    self.context.uniform_2_f32(Some(location.clone()), *x, *y);
                }
                SetUniformValue::Float3(x, y, z) => {
                    self.context
                        .uniform_3_f32(Some(location.clone()), *x, *y, *z);
                }
                SetUniformValue::Float4(x, y, z, w) => {
                    self.context
                        .uniform_4_f32(Some(location.clone()), *x, *y, *z, *w);
                }
                SetUniformValue::Mat2(m) => {
                    self.context.uniform_matrix_2_f32_slice(
                        Some(location.clone()),
                        false,
                        &[m[0][0], m[0][1], m[1][0], m[1][1]],
                    );
                }
                SetUniformValue::Mat3(m) => {
                    self.context.uniform_matrix_3_f32_slice(
                        Some(location.clone()),
                        false,
                        &[
                            m[0][0], m[0][1], m[0][2], m[1][0], m[1][1], m[1][2], m[2][0], m[2][1],
                            m[2][2],
                        ],
                    );
                }
                SetUniformValue::Mat4(m) => {
                    self.context.uniform_matrix_4_f32_slice(
                        Some(location.clone()),
                        false,
                        &[
                            m[0][0], m[0][1], m[0][2], m[0][3], m[1][0], m[1][1], m[1][2], m[1][3],
                            m[2][0], m[2][1], m[2][2], m[2][3], m[3][0], m[3][1], m[3][2], m[3][3],
                        ],
                    );
                }
            }
        }

        for (location, attribute) in self.vertex_format.attributes.iter() {
            self.context.enable_vertex_attrib_array(*location);
            self.context.vertex_attrib_pointer_f32(
                *location,
                attribute.size as i32,
                match attribute.ty {
                    VertexAttributeType::Float => glow::FLOAT,
                    VertexAttributeType::Int => glow::BYTE,
                    VertexAttributeType::Uint => glow::UNSIGNED_BYTE,
                },
                false,
                self.vertex_format.stride,
                attribute.offset as i32,
            );
        }

        self.context
            .draw_arrays(glow::TRIANGLES, 0, vertex_buffer.len as i32);

        Ok(())
    }
}

enum SetUniformValue {
    Texture(Rc<TextureId>),
    Int(i32),
    Int2(i32, i32),
    Int3(i32, i32, i32),
    Int4(i32, i32, i32, i32),
    Float(f32),
    Float2(f32, f32),
    Float3(f32, f32, f32),
    Float4(f32, f32, f32, f32),
    Mat2([[f32; 2]; 2]),
    Mat3([[f32; 3]; 3]),
    Mat4([[f32; 4]; 4]),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UniformType {
    Texture,
    Int,
    Int2,
    Int3,
    Int4,
    Float,
    Float2,
    Float3,
    Float4,
    Mat2,
    Mat3,
    Mat4,
}

pub enum Uniform<'a> {
    Texture(&'a Texture),
    Int(i32),
    Int2(i32, i32),
    Int3(i32, i32, i32),
    Int4(i32, i32, i32, i32),
    Float(f32),
    Float2(f32, f32),
    Float3(f32, f32, f32),
    Float4(f32, f32, f32, f32),
    Mat2([[f32; 2]; 2]),
    Mat3([[f32; 3]; 3]),
    Mat4([[f32; 4]; 4]),
}

impl<'a> Uniform<'a> {
    fn uniform_type(&self) -> UniformType {
        match self {
            Uniform::Texture(_) => UniformType::Texture,
            Uniform::Int(_) => UniformType::Int,
            Uniform::Int2(_, _) => UniformType::Int2,
            Uniform::Int3(_, _, _) => UniformType::Int3,
            Uniform::Int4(_, _, _, _) => UniformType::Int4,
            Uniform::Float(_) => UniformType::Float,
            Uniform::Float2(_, _) => UniformType::Float2,
            Uniform::Float3(_, _, _) => UniformType::Float3,
            Uniform::Float4(_, _, _, _) => UniformType::Float4,
            Uniform::Mat2(_) => UniformType::Mat2,
            Uniform::Mat3(_) => UniformType::Mat3,
            Uniform::Mat4(_) => UniformType::Mat4,
        }
    }
}

#[derive(Clone, Debug)]
pub struct UniformEntry<'a> {
    pub name: &'a str,
    pub ty: UniformType,
}

#[derive(Clone, Copy)]
pub enum VertexAttributeType {
    Int,
    Uint,
    Float,
}

#[derive(Clone)]
pub struct VertexAttribute<'a> {
    pub name: &'a str,
    pub ty: VertexAttributeType,
    pub size: u32,
    pub offset: u32,
}

type VertexAttributeLocation = u32;

pub struct VertexFormat<'a> {
    pub stride: usize,
    pub attributes: &'a [VertexAttribute<'a>],
}

pub struct ProgramDescriptor<'a> {
    pub vertex_shader: &'a Shader,
    pub fragment_shader: &'a Shader,
    pub uniforms: &'a [UniformEntry<'a>],
    pub vertex_format: VertexFormat<'a>,
}
