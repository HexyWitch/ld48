use std::{collections::HashMap, sync::Arc};

use anyhow::Error;
use euclid::{
    default::{Box2D, Point2D, Rect, Transform2D, Vector2D},
    point2, size2, vec2,
};

use crate::{
    constants::{SCREEN_SIZE, TICK_DT, TILE_SIZE, ZOOM_LEVEL},
    gl, graphics,
    graphics::{load_image, load_raw_image, render_sprite, Sprite, Vertex, TEXTURE_ATLAS_SIZE},
    input::{InputEvent, Key},
    mixer::Mixer,
    texture_atlas::{TextureAtlas, TextureRect},
};

pub struct Game {
    program: gl::Program,
    vertex_buffer: gl::VertexBuffer,

    mixer: Arc<Mixer>,

    controls: Controls,
    player: Player,

    rooms: HashMap<RoomColor, Room>,
    room_buffers: HashMap<RoomColor, gl::VertexBuffer>,
    room_blocks: HashMap<RoomColor, TextureRect>,
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

        let transform =
            Transform2D::create_scale(1.0 / SCREEN_SIZE.0 as f32, 1.0 / SCREEN_SIZE.0 as f32)
                .post_scale(ZOOM_LEVEL, ZOOM_LEVEL)
                .post_scale(TILE_SIZE as f32, TILE_SIZE as f32)
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

        let tile_sheet = unsafe {
            load_image(
                include_bytes!("../assets/block.png"),
                &mut atlas,
                &mut texture,
            )
        }
        .unwrap();

        let tile_images = TileImages::new(tile_sheet);

        let mut rooms = HashMap::new();
        let mut room_buffers = HashMap::new();
        let mut room_blocks = HashMap::new();

        let blue_room = parse_room(include_str!("../assets/rooms/blue.rum"));

        let blue_room_block_image = create_room_block(&blue_room, RoomColor::Blue);
        let blue_room_block_texture = unsafe {
            load_raw_image(
                &blue_room_block_image,
                ROOM_BLOCK_IMAGE_SIZE.0,
                ROOM_BLOCK_IMAGE_SIZE.1,
                &mut atlas,
                &mut texture,
            )
            .unwrap()
        };
        room_blocks.insert(RoomColor::Blue, blue_room_block_texture);

        let blue_room_buffer =
            build_room_vertex_buffer(gl_context, &room_blocks, &blue_room, &tile_images);
        room_buffers.insert(RoomColor::Blue, blue_room_buffer);

        rooms.insert(RoomColor::Blue, blue_room);

        let player_rect = unsafe {
            load_image(
                include_bytes!("../assets/player.png"),
                &mut atlas,
                &mut texture,
            )
        }
        .unwrap();

        let player = Player::new(player_rect, point2(5., 5.));

        Game {
            program,
            vertex_buffer,

            mixer,

            controls,
            player,

            rooms,
            room_buffers,
            room_blocks,
        }
    }

    pub fn update(&mut self, inputs: &[InputEvent]) {
        self.controls.jump = false;
        for input in inputs {
            match input {
                InputEvent::KeyDown(Key::W) | InputEvent::KeyDown(Key::Space) => {
                    self.controls.jump = true;
                }
                InputEvent::KeyDown(Key::A) => {
                    self.controls.left = true;
                }
                InputEvent::KeyUp(Key::A) => {
                    self.controls.left = false;
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

        let room = self.rooms.get(&RoomColor::Blue).unwrap();

        // Player controls
        let coyote_time = 0.1;
        let ground_friction = 15.;
        let ground_acc = 100.;
        let air_acc = 25.;
        let run_speed = 6.;
        let fall_speed = 15.;
        let gravity = -30.;
        let jump_speed = 12.;

        let mut x_dir: f32 = 0.;
        if self.controls.right {
            x_dir += 1.;
        }
        if self.controls.left {
            x_dir -= 1.;
        }

        let on_ground = self.player.since_on_ground == 0.;
        if x_dir.abs() > 0. {
            if on_ground {
                if x_dir * self.player.velocity.x < 0. {
                    self.player.velocity.x -= self.player.velocity.x * ground_friction * TICK_DT;
                }
                self.player.velocity.x += x_dir * ground_acc * TICK_DT;
            } else {
                self.player.velocity.x += x_dir * air_acc * TICK_DT;
            }
        } else if on_ground {
            self.player.velocity.x -= self.player.velocity.x * ground_friction * TICK_DT;
        }
        self.player.velocity.x = self.player.velocity.x.min(run_speed).max(-run_speed);
        self.player.velocity.y = self.player.velocity.y.min(fall_speed).max(-fall_speed);

        if self.controls.jump && self.player.since_on_ground < coyote_time {
            self.player.velocity.y = jump_speed;
            self.player.since_on_ground = coyote_time;
        }

        self.player.velocity += vec2(0., gravity) * TICK_DT;

        // Player collision
        self.player.since_on_ground += TICK_DT;
        let mut colliding;

        let mut corrections: Vec<Vector2D<f32>> = Vec::new();
        let mut new_pos = self.player.position + self.player.velocity * TICK_DT;
        let mut i = 0;
        loop {
            i += 1;
            if i > 100 {
                panic!("Physics got straight up broken");
            }
            let player_rect = self.player.collision_rect.translate(new_pos.to_vector());

            let min_x = (player_rect.min_x() + 0.0001).floor() as i32;
            let max_x = (player_rect.max_x() - 0.0001).floor() as i32;
            let min_y = (player_rect.min_y() + 0.0001).floor() as i32;
            let max_y = (player_rect.max_y() - 0.0001).floor() as i32;

            colliding = false;
            corrections.clear();
            for x in min_x..=max_x {
                for y in min_y..=max_y {
                    let collides =
                        if x < 0 || x >= ROOM_SIZE.0 as i32 || y < 0 || y >= ROOM_SIZE.1 as i32 {
                            true
                        } else {
                            let cell = (y * ROOM_SIZE.0 as i32 + x) as usize;
                            room[cell] != Tile::Empty
                        };
                    if collides {
                        let tile_rect = Rect::new(point2(x as f32, y as f32), size2(1., 1.));

                        // push the player right
                        corrections.push(vec2(tile_rect.max_x() - player_rect.min_x(), 0.));
                        // push the player left
                        corrections.push(vec2(tile_rect.min_x() - player_rect.max_x(), 0.));
                        // push the player up
                        corrections.push(vec2(0., tile_rect.max_y() - player_rect.min_y()));
                        // push the player down
                        corrections.push(vec2(0., tile_rect.min_y() - player_rect.max_y()));

                        colliding = true;
                    }
                }
            }

            if !colliding {
                break;
            }

            let mut min_left: f32 = 0.;
            let mut min_right: f32 = 0.;
            let mut min_up: f32 = 0.;
            let mut min_down: f32 = 0.;

            let mut min_correction_by_len = vec2(9999., 9999.);
            for correction in &corrections {
                if correction.x > 0. {
                    min_right = min_right.max(correction.x);
                }
                if correction.x < 0. {
                    min_left = min_left.min(correction.x);
                }
                if correction.y > 0. {
                    min_up = min_up.max(correction.y);
                }
                if correction.y < 0. {
                    min_down = min_down.min(correction.y);
                }
                if correction.length() < min_correction_by_len.length() {
                    min_correction_by_len = *correction;
                }
            }

            let mut min_correction: f32 = 9999.;
            let mut correction_vec = vec2(0., 0.);
            for (mag, correction) in &[
                (min_left.abs(), vec2(min_left, 0.)),
                (min_right.abs(), vec2(min_right, 0.)),
                (min_up.abs(), vec2(0., min_up)),
                (min_down.abs(), vec2(0., min_down)),
            ] {
                if *mag < min_correction {
                    correction_vec = *correction;
                }
                min_correction = min_correction.min(*mag);
            }

            if min_correction.abs() < 1.0 {
                new_pos += correction_vec;
            } else {
                new_pos += min_correction_by_len;
            }

            if correction_vec.y > 0. {
                self.player.since_on_ground = 0.;
            }

            if correction_vec.x.abs() > 0. {
                self.player.velocity.x = 0.;
            } else {
                self.player.velocity.y = 0.;
            }
        }

        self.player.position = new_pos;
    }

    pub fn draw(&mut self, context: &mut gl::Context) {
        let mut entity_vertices = Vec::new();
        render_sprite(
            &self.player.sprite,
            0,
            self.player.position,
            &mut entity_vertices,
        );

        unsafe {
            context.clear([0.5, 0.5, 0.8, 1.0]);

            self.vertex_buffer.write(&entity_vertices);
            self.program.render_vertices(&self.vertex_buffer).unwrap();

            self.program
                .render_vertices(self.room_buffers.get(&RoomColor::Blue).as_ref().unwrap())
                .unwrap();
        }
    }
}

struct TileImages {
    // top left
    tl_outer_corner: TextureRect,
    tl_horz: TextureRect,
    tl_vert: TextureRect,
    tl_solid: TextureRect,

    // top right
    tr_outer_corner: TextureRect,
    tr_horz: TextureRect,
    tr_vert: TextureRect,
    tr_solid: TextureRect,

    // bottom left
    bl_outer_corner: TextureRect,
    bl_horz: TextureRect,
    bl_vert: TextureRect,
    bl_solid: TextureRect,

    // bottom right
    br_outer_corner: TextureRect,
    br_horz: TextureRect,
    br_vert: TextureRect,
    br_solid: TextureRect,
}

impl TileImages {
    pub fn new(tex: TextureRect) -> TileImages {
        let to_origin = vec2(tex[0], tex[1]);
        let tl_rect = Rect::new(point2(0, 0) + to_origin, size2(8, 8));
        let tr_rect = Rect::new(point2(8, 0) + to_origin, size2(7, 8));
        let bl_rect = Rect::new(point2(0, 8) + to_origin, size2(8, 7));
        let br_rect = Rect::new(point2(8, 8) + to_origin, size2(7, 7));
        let to_texture_rect = |rect: Rect<u32>| -> TextureRect {
            [rect.min_x(), rect.min_y(), rect.max_x(), rect.max_y()]
        };

        TileImages {
            tl_outer_corner: to_texture_rect(tl_rect),
            tl_horz: to_texture_rect(tl_rect.translate(vec2(15, 0))),
            tl_vert: to_texture_rect(tl_rect.translate(vec2(30, 0))),
            tl_solid: to_texture_rect(tl_rect.translate(vec2(45, 0))),

            tr_outer_corner: to_texture_rect(tr_rect),
            tr_horz: to_texture_rect(tr_rect.translate(vec2(15, 0))),
            tr_vert: to_texture_rect(tr_rect.translate(vec2(30, 0))),
            tr_solid: to_texture_rect(tr_rect.translate(vec2(45, 0))),

            bl_outer_corner: to_texture_rect(bl_rect),
            bl_horz: to_texture_rect(bl_rect.translate(vec2(15, 0))),
            bl_vert: to_texture_rect(bl_rect.translate(vec2(30, 0))),
            bl_solid: to_texture_rect(bl_rect.translate(vec2(45, 0))),

            br_outer_corner: to_texture_rect(br_rect),
            br_horz: to_texture_rect(br_rect.translate(vec2(15, 0))),
            br_vert: to_texture_rect(br_rect.translate(vec2(30, 0))),
            br_solid: to_texture_rect(br_rect.translate(vec2(45, 0))),
        }
    }
}

fn build_room_vertex_buffer(
    gl_context: &mut gl::Context,
    room_block_textures: &HashMap<RoomColor, TextureRect>,
    room: &[Tile],
    tile_images: &TileImages,
) -> gl::VertexBuffer {
    let mut vertices: Vec<Vertex> = Vec::with_capacity(ROOM_CELLS as usize * 4 * 4);
    let get_tile = |x: i32, y: i32| -> Tile {
        if x < 0 || x >= ROOM_SIZE.0 as i32 || y < 0 || y >= ROOM_SIZE.1 as i32 {
            Tile::Solid
        } else {
            let cell = (y as u32 * ROOM_SIZE.0 + x as u32) as usize;
            room[cell]
        }
    };

    let mut room_blocks = Vec::new();
    for (cell, tile) in room.iter().enumerate() {
        let y = (cell as u32 / ROOM_SIZE.0) as i32;
        let x = (cell as u32 % ROOM_SIZE.0) as i32;
        if *tile == Tile::Empty {
            continue;
        }

        // draw room blocks later
        match tile {
            Tile::Room(color) => {
                room_blocks.push(((x, y), color));
                continue;
            }
            _ => {}
        }

        let (t, l, r, b) = (
            get_tile(x, y + 1),
            get_tile(x - 1, y),
            get_tile(x + 1, y),
            get_tile(x, y - 1),
        );

        let rect = Rect::new(point2(x as f32, y as f32), size2(1., 1.));
        let mid = Point2D::new(x as f32 + (8. / TILE_SIZE), y as f32 + (7. / TILE_SIZE));

        // top left rect
        let tl_box = Box2D::new(point2(rect.min_x(), mid.y), point2(mid.x, rect.max_y()));
        if l != Tile::Solid && t != Tile::Solid {
            graphics::render_quad(tl_box, tile_images.tl_outer_corner, &mut vertices);
        } else if l == Tile::Solid && t != Tile::Solid {
            graphics::render_quad(tl_box, tile_images.tl_horz, &mut vertices);
        } else if l != Tile::Solid && t == Tile::Solid {
            graphics::render_quad(tl_box, tile_images.tl_vert, &mut vertices);
        } else {
            graphics::render_quad(tl_box, tile_images.tl_solid, &mut vertices);
        }

        // top right rect
        let tr_box = Box2D::new(point2(mid.x, mid.y), point2(rect.max_x(), rect.max_y()));
        if r != Tile::Solid && t != Tile::Solid {
            graphics::render_quad(tr_box, tile_images.tr_outer_corner, &mut vertices);
        } else if r == Tile::Solid && t != Tile::Solid {
            graphics::render_quad(tr_box, tile_images.tr_horz, &mut vertices);
        } else if r != Tile::Solid && t == Tile::Solid {
            graphics::render_quad(tr_box, tile_images.tr_vert, &mut vertices);
        } else {
            graphics::render_quad(tr_box, tile_images.tr_solid, &mut vertices);
        }

        // bottom left rect
        let bl_box = Box2D::new(rect.min(), mid);
        if l != Tile::Solid && b != Tile::Solid {
            graphics::render_quad(bl_box, tile_images.bl_outer_corner, &mut vertices);
        } else if l == Tile::Solid && b != Tile::Solid {
            graphics::render_quad(bl_box, tile_images.bl_horz, &mut vertices);
        } else if l != Tile::Solid && b == Tile::Solid {
            graphics::render_quad(bl_box, tile_images.bl_vert, &mut vertices);
        } else {
            graphics::render_quad(bl_box, tile_images.bl_solid, &mut vertices);
        }

        // bottom right rect
        let br_box = Box2D::new(point2(mid.x, rect.min_y()), point2(rect.max_x(), mid.y));
        if r != Tile::Solid && b != Tile::Solid {
            graphics::render_quad(br_box, tile_images.br_outer_corner, &mut vertices);
        } else if r == Tile::Solid && b != Tile::Solid {
            graphics::render_quad(br_box, tile_images.br_horz, &mut vertices);
        } else if r != Tile::Solid && b == Tile::Solid {
            graphics::render_quad(br_box, tile_images.br_vert, &mut vertices);
        } else {
            graphics::render_quad(br_box, tile_images.br_solid, &mut vertices);
        }
    }

    for ((x, y), color) in room_blocks {
        let room_block_box = Box2D::new(
            point2(x as f32 - 1. / TILE_SIZE, y as f32 - 1. / TILE_SIZE),
            point2(
                (x + 1) as f32 + 1. / TILE_SIZE,
                (y + 1) as f32 + 1. / TILE_SIZE,
            ),
        );
        graphics::render_quad(
            room_block_box,
            *room_block_textures.get(color).unwrap(),
            &mut vertices,
        );
    }

    unsafe {
        let mut buffer = gl_context.create_vertex_buffer().unwrap();
        buffer.write(&vertices);
        buffer
    }
}

fn create_room_block(room: &Room, color: RoomColor) -> Vec<u8> {
    let colors = room_block_colors(color);

    let mut image =
        vec![0; ROOM_BLOCK_IMAGE_SIZE.0 as usize * ROOM_BLOCK_IMAGE_SIZE.1 as usize * 4];
    let mut set_pixel = |x: u32, y: u32, color: (u8, u8, u8)| {
        let y = ROOM_BLOCK_IMAGE_SIZE.1 - 1 - y;
        let index = (y * ROOM_BLOCK_IMAGE_SIZE.0 + x) as usize * 4;
        image[index] = color.0;
        image[index + 1] = color.1;
        image[index + 2] = color.2;
        image[index + 3] = 255;
    };

    let get_tile = |x: i32, y: i32| -> Tile {
        if x < 0 || x >= ROOM_SIZE.0 as i32 || y < 0 || y >= ROOM_SIZE.1 as i32 {
            Tile::Solid
        } else {
            let cell = (y as u32 * ROOM_SIZE.0 + x as u32) as usize;
            room[cell]
        }
    };
    let tile_at = |x: i32, y: i32| -> bool { get_tile(x, y) != Tile::Empty };

    for x in 0..ROOM_BLOCK_IMAGE_SIZE.0 {
        for y in 0..ROOM_BLOCK_IMAGE_SIZE.1 {
            let tile_x = x as i32 - 1;
            let tile_y = y as i32 - 1;

            if x < 1 && y >= 1 && y < ROOM_BLOCK_IMAGE_SIZE.1 - 1 && tile_at(tile_x + 1, tile_y) {
                set_pixel(x, y, colors.outer_border);
            } else if x > ROOM_SIZE.0
                && y >= 1
                && y < ROOM_BLOCK_IMAGE_SIZE.1 - 1
                && tile_at(tile_x - 1, tile_y)
            {
                set_pixel(x, y, colors.outer_border);
            } else if y < 1
                && x >= 1
                && x < ROOM_BLOCK_IMAGE_SIZE.0 - 1
                && tile_at(tile_x, tile_y + 1)
            {
                set_pixel(x, y, colors.outer_border);
            } else if y > ROOM_SIZE.1
                && x >= 1
                && x < ROOM_BLOCK_IMAGE_SIZE.0 - 1
                && tile_at(tile_x, tile_y + 1)
            {
                set_pixel(x, y, colors.outer_border);
            }

            if x > 0 && x - 1 < ROOM_SIZE.0 && y > 0 && y - 1 < ROOM_SIZE.1 {
                match get_tile(tile_x, tile_y) {
                    Tile::Empty => set_pixel(x, y, colors.background),
                    Tile::Solid => {
                        if tile_at(tile_x - 1, tile_y + 1)
                            && tile_at(tile_x, tile_y + 1)
                            && tile_at(tile_x + 1, tile_y + 1)
                            && tile_at(tile_x - 1, tile_y)
                            && tile_at(tile_x + 1, tile_y)
                            && tile_at(tile_x - 1, tile_y - 1)
                            && tile_at(tile_x, tile_y - 1)
                            && tile_at(tile_x + 1, tile_y - 1)
                        {
                            set_pixel(x, y, colors.inner);
                        } else {
                            set_pixel(x, y, colors.border);
                        }
                    }
                    Tile::Room(color) => set_pixel(x, y, room_block_colors(color).border),
                }
            }
        }
    }

    image
}

#[derive(Default)]
struct Controls {
    left: bool,
    right: bool,
    jump: bool,
}

struct Player {
    position: Point2D<f32>,
    velocity: Vector2D<f32>,

    since_on_ground: f32,

    sprite: Sprite,

    collision_rect: Rect<f32>,
}

impl Player {
    pub fn new(texture: TextureRect, position: Point2D<f32>) -> Player {
        let mut player_sprite = Sprite::new(texture, 1, point2(0., 0.));
        player_sprite.set_transform(
            Transform2D::create_translation(-7.5, -7.5).post_scale(1. / TILE_SIZE, 1. / TILE_SIZE),
        );

        Player {
            position,
            velocity: vec2(0., 0.),

            since_on_ground: 9999.,

            sprite: player_sprite,

            collision_rect: Rect::new(
                point2(-3.0 / TILE_SIZE, -7.5 / TILE_SIZE),
                size2(6. / TILE_SIZE, 14. / TILE_SIZE),
            ),
        }
    }
}

const ROOM_SIZE: (u32, u32) = (15, 15);
// ROOM_SIZE.0 * ROOM_SIZE.1
const ROOM_CELLS: usize = 225;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tile {
    Empty,
    Solid,
    Room(RoomColor),
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum RoomColor {
    Blue,
}

const ROOM_BLOCK_IMAGE_SIZE: (u32, u32) = (17, 17);

struct RoomBlockColors {
    background: (u8, u8, u8),
    inner: (u8, u8, u8),
    border: (u8, u8, u8),
    outer_border: (u8, u8, u8),
}

fn room_block_colors(color: RoomColor) -> RoomBlockColors {
    match color {
        RoomColor::Blue => RoomBlockColors {
            background: (141, 137, 196),
            inner: (78, 83, 148),
            border: (55, 58, 103),
            outer_border: (33, 35, 63),
        },
    }
}

type Room = [Tile; ROOM_CELLS];

fn parse_room(level: &str) -> [Tile; ROOM_CELLS] {
    let mut tiles = [Tile::Empty; ROOM_CELLS];
    for (y, line) in level.lines().enumerate() {
        for (x, c) in line.chars().enumerate() {
            // flip y
            let y = ROOM_SIZE.1 as usize - 1 - y;
            let cell = y * ROOM_SIZE.0 as usize + x;
            tiles[cell] = match c {
                ' ' => Tile::Empty,
                '#' => Tile::Solid,
                'B' => Tile::Room(RoomColor::Blue),
                c @ _ => {
                    panic!("Unrecognized tile identifier '{}'", c);
                }
            }
        }
    }
    tiles
}
