#[allow(unused)]
mod gl;
mod graphics;
mod input;
mod mixer;
mod platform;
mod texture_atlas;

use euclid::{
    default::{Transform2D, Vector2D},
    point2, vec2,
};

use graphics::{load_image, render_sprite, Sprite, Vertex, TEXTURE_ATLAS_SIZE};
use input::{InputEvent, Key};

fn main() {
    platform::run(
        "Ludum Dare 48",
        (800, 600),
        |gl_context: &mut gl::Context| {
            let vertex_shader = unsafe {
                gl_context
                    .create_shader(gl::ShaderType::Vertex, include_str!("shaders/shader.vert"))
                    .unwrap()
            };
            let fragment_shader = unsafe {
                gl_context
                    .create_shader(
                        gl::ShaderType::Fragment,
                        include_str!("shaders/shader.frag"),
                    )
                    .unwrap()
            };

            let mut program = unsafe {
                gl_context
                    .create_program(&gl::ProgramDescriptor {
                        vertex_shader: &vertex_shader,
                        fragment_shader: &fragment_shader,
                        uniforms: &[
                            gl::UniformEntry {
                                name: "u_transform",
                                ty: gl::UniformType::Mat3,
                            },
                            gl::UniformEntry {
                                name: "u_texture",
                                ty: gl::UniformType::Texture,
                            },
                        ],
                        vertex_format: gl::VertexFormat {
                            stride: std::mem::size_of::<Vertex>(),
                            attributes: &[
                                gl::VertexAttribute {
                                    name: "a_pos",
                                    ty: gl::VertexAttributeType::Float,
                                    size: 2,
                                    offset: 0,
                                },
                                gl::VertexAttribute {
                                    name: "a_uv",
                                    ty: gl::VertexAttributeType::Float,
                                    size: 2,
                                    offset: 2 * 4,
                                },
                            ],
                        },
                    })
                    .unwrap()
            };

            let mut texture = unsafe {
                gl_context
                    .create_texture(
                        gl::TextureFormat::RGBAFloat,
                        TEXTURE_ATLAS_SIZE.width,
                        TEXTURE_ATLAS_SIZE.height,
                    )
                    .unwrap()
            };
            let mut atlas = texture_atlas::TextureAtlas::new((
                TEXTURE_ATLAS_SIZE.width,
                TEXTURE_ATLAS_SIZE.height,
            ));

            let logo_rect = unsafe {
                load_image(
                    include_bytes!("../assets/embla_logo.png"),
                    &mut atlas,
                    &mut texture,
                )
            }
            .unwrap();

            let logo_sprite = Sprite::new(logo_rect, 1, point2(0., 0.));

            let transform = Transform2D::create_scale(1.0 / 800.0, 1.0 / 600.0)
                .post_scale(2., 2.)
                .post_translate(vec2(-1.0, -1.0));
            program
                .set_uniform(
                    0,
                    gl::Uniform::Mat3([
                        [transform.m11, transform.m12, 0.0],
                        [transform.m21, transform.m22, 0.0],
                        [transform.m31, transform.m32, 1.0],
                    ]),
                )
                .unwrap();

            program
                .set_uniform(1, gl::Uniform::Texture(&texture))
                .unwrap();

            let mut vertex_buffer = unsafe { gl_context.create_vertex_buffer().unwrap() };
            let mut position = point2(50., 100.);

            #[derive(Default)]
            struct Controls {
                up: bool,
                left: bool,
                down: bool,
                right: bool,
            }
            let mut controls = Controls::default();
            move |dt: f32, inputs: &[InputEvent], gl_context: &mut gl::Context| {
                for input in inputs {
                    match input {
                        InputEvent::KeyDown(Key::W) => {
                            controls.up = true;
                        }
                        InputEvent::KeyUp(Key::W) => {
                            controls.up = false;
                        }
                        InputEvent::KeyDown(Key::A) => {
                            controls.left = true;
                        }
                        InputEvent::KeyUp(Key::A) => {
                            controls.left = false;
                        }
                        InputEvent::KeyDown(Key::S) => {
                            controls.down = true;
                        }
                        InputEvent::KeyUp(Key::S) => {
                            controls.down = false;
                        }
                        InputEvent::KeyDown(Key::D) => {
                            controls.right = true;
                        }
                        InputEvent::KeyUp(Key::D) => {
                            controls.right = false;
                        }
                        _ => {}
                    }
                }

                let mut dir: Vector2D<f32> = vec2(0., 0.);
                if controls.up {
                    dir.y += 1.;
                }
                if controls.down {
                    dir.y -= 1.;
                }
                if controls.right {
                    dir.x += 1.;
                }
                if controls.left {
                    dir.x -= 1.;
                }
                if dir.length() > 0. {
                    position += dir.normalize() * 100. * dt;
                }

                let mut vertices = Vec::new();
                render_sprite(&logo_sprite, 0, position, &mut vertices);

                unsafe {
                    gl_context.clear([0.0, 0.0, 0.0, 1.0]);

                    vertex_buffer.write(&vertices);

                    program.render_vertices(&vertex_buffer).unwrap();
                }
            }
        },
    )
}
