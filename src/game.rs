use std::sync::Arc;

use anyhow::Error;
use euclid::{
    default::{Point2D, Transform2D, Vector2D},
    point2, vec2,
};

use crate::{
    constants::{SCREEN_SIZE, TICK_DT, ZOOM_LEVEL},
    gl,
    graphics::{load_image, render_sprite, Sprite, Vertex, TEXTURE_ATLAS_SIZE},
    input::{InputEvent, Key},
    mixer::Mixer,
    texture_atlas::TextureAtlas,
};

pub struct Game {
    program: gl::Program,
    vertex_buffer: gl::VertexBuffer,

    mixer: Arc<Mixer>,

    controls: Controls,
    player_sprite: Sprite,
    player: Player,
}

impl Game {
    pub fn new(gl_context: &mut gl::Context, mixer: Arc<Mixer>) -> Self {
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
        let mut atlas = TextureAtlas::new((TEXTURE_ATLAS_SIZE.width, TEXTURE_ATLAS_SIZE.height));

        let player_rect = unsafe {
            load_image(
                include_bytes!("../assets/player.png"),
                &mut atlas,
                &mut texture,
            )
        }
        .unwrap();

        let player_sprite = Sprite::new(player_rect, 1, point2(0., 0.));

        let transform =
            Transform2D::create_scale(1.0 / SCREEN_SIZE.0 as f32, 1.0 / SCREEN_SIZE.0 as f32)
                .post_scale(ZOOM_LEVEL, ZOOM_LEVEL)
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

        let vertex_buffer = unsafe { gl_context.create_vertex_buffer().unwrap() };

        let controls = Controls::default();

        Game {
            program,
            vertex_buffer,

            mixer,

            controls,
            player_sprite,
            player: Player {
                position: point2(100., 100.),
            },
        }
    }

    pub fn update(&mut self, inputs: &[InputEvent]) {
        for input in inputs {
            match input {
                InputEvent::KeyDown(Key::W) => {
                    self.controls.up = true;
                }
                InputEvent::KeyUp(Key::W) => {
                    self.controls.up = false;
                }
                InputEvent::KeyDown(Key::A) => {
                    self.controls.left = true;
                }
                InputEvent::KeyUp(Key::A) => {
                    self.controls.left = false;
                }
                InputEvent::KeyDown(Key::S) => {
                    self.controls.down = true;
                }
                InputEvent::KeyUp(Key::S) => {
                    self.controls.down = false;
                }
                InputEvent::KeyDown(Key::D) => {
                    self.controls.right = true;
                }
                InputEvent::KeyUp(Key::D) => {
                    self.controls.right = false;
                }
                _ => {}
            }
        }

        let mut dir: Vector2D<f32> = vec2(0., 0.);
        if self.controls.up {
            dir.y += 1.;
        }
        if self.controls.down {
            dir.y -= 1.;
        }
        if self.controls.right {
            dir.x += 1.;
        }
        if self.controls.left {
            dir.x -= 1.;
        }

        if dir.length() > 0. {
            self.player.position += dir.normalize() * 100. * TICK_DT;
        }
    }

    pub fn draw(&mut self, context: &mut gl::Context) {
        let mut vertices = Vec::new();
        render_sprite(&self.player_sprite, 0, self.player.position, &mut vertices);

        unsafe {
            context.clear([0.5, 0.5, 0.8, 1.0]);

            self.vertex_buffer.write(&vertices);

            self.program.render_vertices(&self.vertex_buffer).unwrap();
        }
    }
}

#[derive(Default)]
struct Controls {
    up: bool,
    left: bool,
    down: bool,
    right: bool,
}

struct Player {
    position: Point2D<f32>,
}
