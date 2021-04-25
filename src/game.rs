use std::{collections::HashMap, sync::Arc};

use euclid::{
    default::{Box2D, Point2D, Rect, Size2D, Transform2D, Vector2D},
    point2, size2, vec2,
};
use palette::{Hsv, LinSrgb};

use crate::{
    constants::{MUSIC_VOLUME, SCREEN_SIZE, TICK_DT, TILE_SIZE, ZOOM_LEVEL},
    gl, graphics,
    graphics::{load_image, load_raw_image, render_sprite, Sprite, Vertex, TEXTURE_ATLAS_SIZE},
    input::{InputEvent, Key, MouseButton},
    mixer::{Audio, AudioInstanceHandle, Mixer},
    texture_atlas::{TextureAtlas, TextureRect},
};

pub struct Game {
    program: gl::Program,
    room_vertex_buffer: gl::VertexBuffer,
    vertex_buffer: gl::VertexBuffer,
    ui_buffer: gl::VertexBuffer,
    atlas_texture: gl::Texture,

    mixer: Arc<Mixer>,
    run_sound: Audio,
    run_handle: Option<AudioInstanceHandle>,
    jump_sound: Audio,
    land_sound: Audio,
    stop_sound: Audio,
    enter_sound: Audio,

    music_handle: AudioInstanceHandle,

    mouse_pos: Point2D<f32>,
    muted: bool,
    mute_icon_rect: Rect<f32>,
    mute_icon: Sprite,

    controls: Controls,
    player: Player,

    rooms: HashMap<RoomColor, Room>,
    room_textures: HashMap<RoomColor, gl::Texture>,

    current_room: RoomColor,
    enter_room: Option<RoomTransitionIn>,
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
                        gl::UniformEntry {
                            name: "u_alpha",
                            ty: gl::UniformType::Float,
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
                            gl::VertexAttribute {
                                name: "a_color",
                                ty: gl::VertexAttributeType::Float,
                                size: 4,
                                offset: 4 * 4,
                            },
                        ],
                    },
                })
                .unwrap()
        };

        let mut atlas_texture = unsafe {
            gl_context
                .create_texture(
                    gl::TextureFormat::RGBAFloat,
                    TEXTURE_ATLAS_SIZE.width,
                    TEXTURE_ATLAS_SIZE.height,
                )
                .unwrap()
        };
        let mut atlas = TextureAtlas::new((TEXTURE_ATLAS_SIZE.width, TEXTURE_ATLAS_SIZE.height));

        let vertex_buffer = unsafe { gl_context.create_vertex_buffer().unwrap() };
        let ui_buffer = unsafe { gl_context.create_vertex_buffer().unwrap() };

        let mut room_vertex_buffer = unsafe { gl_context.create_vertex_buffer().unwrap() };
        let room_vertices = vec![
            Vertex {
                position: [0.0, 0.0],
                uv: [0.0, 0.0],
                color: [1., 1., 1., 1.],
            },
            Vertex {
                position: [ROOM_SIZE.0 as f32, 0.0],
                uv: [1.0, 0.0],
                color: [1., 1., 1., 1.],
            },
            Vertex {
                position: [0.0, ROOM_SIZE.1 as f32],
                uv: [0.0, 1.0],
                color: [1., 1., 1., 1.],
            },
            Vertex {
                position: [ROOM_SIZE.0 as f32, 0.0],
                uv: [1.0, 0.0],
                color: [1., 1., 1., 1.],
            },
            Vertex {
                position: [ROOM_SIZE.0 as f32, ROOM_SIZE.1 as f32],
                uv: [1.0, 1.0],
                color: [1., 1., 1., 1.],
            },
            Vertex {
                position: [0.0, ROOM_SIZE.1 as f32],
                uv: [0.0, 1.0],
                color: [1., 1., 1., 1.],
            },
        ];
        unsafe { room_vertex_buffer.write(&room_vertices) };

        let controls = Controls::default();

        let tile_sheet = unsafe {
            load_image(
                include_bytes!("../assets/block.png"),
                &mut atlas,
                &mut atlas_texture,
            )
        }
        .unwrap();

        let tile_images = TileImages::new(tile_sheet);

        let mut rooms = HashMap::new();
        let mut room_textures = HashMap::new();
        let mut room_blocks = HashMap::new();

        let room_list = vec![
            (
                RoomColor::Red,
                parse_room(include_str!("../assets/rooms/red.rum")),
            ),
            (
                RoomColor::Orange,
                parse_room(include_str!("../assets/rooms/orange.rum")),
            ),
            (
                RoomColor::Yellow,
                parse_room(include_str!("../assets/rooms/yellow.rum")),
            ),
            (
                RoomColor::Green,
                parse_room(include_str!("../assets/rooms/green.rum")),
            ),
            (
                RoomColor::Turquoise,
                parse_room(include_str!("../assets/rooms/turquoise.rum")),
            ),
            (
                RoomColor::Aqua,
                parse_room(include_str!("../assets/rooms/aqua.rum")),
            ),
            (
                RoomColor::Chetwood,
                parse_room(include_str!("../assets/rooms/chetwood.rum")),
            ),
            (
                RoomColor::Blue,
                parse_room(include_str!("../assets/rooms/blue.rum")),
            ),
            (
                RoomColor::Purple,
                parse_room(include_str!("../assets/rooms/purple.rum")),
            ),
            (
                RoomColor::Magenta,
                parse_room(include_str!("../assets/rooms/magenta.rum")),
            ),
            (
                RoomColor::Ferrish,
                parse_room(include_str!("../assets/rooms/ferrish.rum")),
            ),
        ];

        // first create  room blocks
        for (color, room) in &room_list {
            let room_block_image = create_room_block(&room, *color);
            let room_block_texture = unsafe {
                load_raw_image(
                    &room_block_image,
                    ROOM_BLOCK_IMAGE_SIZE.0,
                    ROOM_BLOCK_IMAGE_SIZE.1,
                    &mut atlas,
                    &mut atlas_texture,
                )
                .unwrap()
            };
            room_blocks.insert(*color, room_block_texture);
        }

        for (color, room) in room_list {
            let room_buffer =
                build_room_vertex_buffer(gl_context, &room_blocks, color, &room, &tile_images);
            let room_pixel_size = Size2D::new(ROOM_SIZE.0, ROOM_SIZE.1).to_f32() * TILE_SIZE;
            let transform = Transform2D::scale(
                1.0 / room_pixel_size.width as f32,
                1.0 / room_pixel_size.height as f32,
            )
            .then_scale(TILE_SIZE as f32, TILE_SIZE as f32)
            .then_scale(2., 2.)
            .then_translate(vec2(-1.0, -1.0));
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
                .set_uniform(1, gl::Uniform::Texture(&atlas_texture))
                .unwrap();
            program.set_uniform(2, gl::Uniform::Float(1.0)).unwrap();

            unsafe {
                let room_texture = gl_context
                    .create_texture(
                        gl::TextureFormat::RGBAFloat,
                        room_pixel_size.width as u32,
                        room_pixel_size.height as u32,
                    )
                    .unwrap();
                let room_render_target = gl_context.create_texture_render_target(&room_texture);

                program
                    .render_vertices(&room_buffer, gl::RenderTarget::Texture(&room_render_target))
                    .unwrap();
                room_textures.insert(color, room_texture);
            }

            rooms.insert(color, room);
        }

        let player_rect = unsafe {
            load_image(
                include_bytes!("../assets/player.png"),
                &mut atlas,
                &mut atlas_texture,
            )
        }
        .unwrap();

        let player = Player::new(player_rect, point2(2., 2.));

        let run_sound = mixer.load_ogg(include_bytes!("../assets/run.ogg")).unwrap();
        let jump_sound = mixer
            .load_ogg(include_bytes!("../assets/jump.ogg"))
            .unwrap();
        let land_sound = mixer
            .load_ogg(include_bytes!("../assets/land.ogg"))
            .unwrap();
        let stop_sound = mixer
            .load_ogg(include_bytes!("../assets/stop.ogg"))
            .unwrap();
        let enter_sound = mixer
            .load_ogg(include_bytes!("../assets/enter.ogg"))
            .unwrap();
        let music_sound = mixer
            .load_ogg(include_bytes!("../assets/music.ogg"))
            .unwrap();

        let music_handle = mixer.play(&music_sound, MUSIC_VOLUME, true);

        let mute_texture = unsafe {
            load_image(
                include_bytes!("../assets/music_icon.png"),
                &mut atlas,
                &mut atlas_texture,
            )
            .unwrap()
        };

        let ui_zoom = 2.;
        let mut mute_icon = Sprite::new(mute_texture, 2, point2(0.0, 0.0));
        mute_icon.set_transform(Transform2D::scale(ui_zoom, ui_zoom));
        let mute_icon_rect = Rect::new(
            point2(8., SCREEN_SIZE.1 as f32 - 8. - 11. * ui_zoom),
            size2(9., 11.) * ui_zoom,
        );

        Game {
            program,
            room_vertex_buffer,
            vertex_buffer,
            ui_buffer,
            atlas_texture,

            mixer,
            run_sound,
            run_handle: None,
            jump_sound,
            land_sound,
            stop_sound,
            enter_sound,

            music_handle,

            mouse_pos: Point2D::zero(),
            muted: false,
            mute_icon_rect,
            mute_icon,

            controls,
            player,

            rooms,
            room_textures,

            current_room: RoomColor::Blue,
            enter_room: None,
        }
    }

    pub fn update(&mut self, inputs: &[InputEvent]) {
        for input in inputs {
            match input {
                InputEvent::KeyDown(Key::W) | InputEvent::KeyDown(Key::Space) => {
                    self.controls.since_jump = 0.0;
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
                InputEvent::MouseMove(position) => {
                    self.mouse_pos = point2(position.x, SCREEN_SIZE.1 as f32 - position.y);
                }
                InputEvent::MouseDown(button) => {
                    if let MouseButton::Left = button {
                        if self.mute_icon_rect.contains(self.mouse_pos) {
                            self.muted = !self.muted;
                            if self.muted {
                                self.mixer.set_volume(&self.music_handle, 0.);
                            } else {
                                self.mixer.set_volume(&self.music_handle, MUSIC_VOLUME)
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if let Some(enter_room) = &mut self.enter_room {
            enter_room.timer += TICK_DT;
            if enter_room.timer > ENTER_ROOM_TIME {
                self.current_room = enter_room.color;
                let player_offset = vec2(0.5, -self.player.collision_rect.min_y());
                self.player.position = match enter_room.entrance {
                    RoomEntrance::Left => {
                        self.rooms
                            .get(&enter_room.color)
                            .unwrap()
                            .left_entrance
                            .unwrap()
                            .to_f32()
                            + player_offset
                    }
                    RoomEntrance::Top => {
                        self.rooms
                            .get(&enter_room.color)
                            .unwrap()
                            .top_entrance
                            .unwrap()
                            .to_f32()
                            + player_offset
                    }
                    RoomEntrance::Right => {
                        self.rooms
                            .get(&enter_room.color)
                            .unwrap()
                            .right_entrance
                            .unwrap()
                            .to_f32()
                            + player_offset
                    }
                };
                self.player.velocity = Vector2D::zero();
                self.enter_room = None;
            } else {
                return;
            }
        }

        let room = self.rooms.get(&self.current_room).unwrap();

        // Player controls
        let coyote_time = 0.1;
        let jump_buffer_time = 0.05;
        let ground_friction = 15.;
        let ground_acc = 100.;
        let air_acc = 25.;
        let run_speed = 6.;
        let fall_speed = 15.;
        let gravity = -30.;
        let jump_speed = 11.5;

        let mut x_dir: f32 = 0.;
        if self.controls.right {
            x_dir += 1.;
        }
        if self.controls.left {
            x_dir -= 1.;
        }

        if x_dir.abs() > 0.0001 && self.player.velocity.x.abs() > 0. {
            if self.player.animation_timer < 0. {
                self.player.animation_timer = 0.;
            }
            self.player.flip = x_dir < 0.;

            self.player.animation_timer =
                (self.player.animation_timer + TICK_DT) % RUN_ANIMATION_TIME;
        } else {
            self.player.animation_timer = -1.;
        }

        let on_ground = self.player.since_on_ground == 0.;

        if x_dir.abs() > 0.0001 && self.player.velocity.x.abs() > 0. && on_ground {
            if self.run_handle.is_none() {
                self.run_handle = Some(self.mixer.play(&self.run_sound, 1.0, true));
            }
        } else {
            if let Some(handle) = self.run_handle.take() {
                if on_ground {
                    self.mixer.play(&self.stop_sound, 0.5, false);
                }
                self.mixer.set_looping(&handle, false);
            }
        }

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

        let jumped = self.controls.since_jump < jump_buffer_time;
        if jumped && self.player.since_on_ground < coyote_time {
            self.mixer.play(&self.jump_sound, 1.0, false);

            self.player.velocity.y = jump_speed;
            self.controls.since_jump = jump_buffer_time;
            self.player.since_on_ground = coyote_time;
        }

        self.player.velocity += vec2(0., gravity) * TICK_DT;

        self.player.since_on_ground += TICK_DT;
        self.controls.since_jump += TICK_DT;

        // Player collision
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

            colliding = false;
            corrections.clear();

            let shrunk_player_rect = Rect::new(
                player_rect.origin + vec2(0.0001, 0.0001),
                player_rect.size - size2(0.0002, 0.002),
            );
            room.for_each_tile_in_rect(shrunk_player_rect, |pos, tile| {
                if tile != Tile::Empty {
                    let tile_rect = Rect::new(point2(pos.x as f32, pos.y as f32), size2(1., 1.));

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
            });

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

        if !on_ground && self.player.since_on_ground == 0. {
            self.mixer.play(&self.land_sound, 1.0, false);
        }

        self.player.position = new_pos;

        // Player block interaction
        let player_interact_rect = self
            .player
            .interact_rect
            .translate(self.player.position.to_vector());

        let enter_room = &mut self.enter_room;
        let player_position = self.player.position;
        let rooms = &self.rooms;
        let mut entered = false;
        room.for_each_tile_in_rect(player_interact_rect, |pos, tile| {
            if let Tile::Room(color) = tile {
                let left_enter_region = Rect::new(pos.to_f32() + vec2(-1., 0.), size2(1., 1.));
                if left_enter_region.contains(player_position) {
                    if rooms.get(&color).unwrap().left_entrance.is_some() {
                        *enter_room = Some(RoomTransitionIn {
                            position: pos,
                            entrance: RoomEntrance::Left,
                            color,
                            timer: 0.,
                        });
                    }
                }

                let top_enter_region = Rect::new(pos.to_f32() + vec2(0., 1.), size2(1., 1.));
                if top_enter_region.contains(player_position) {
                    if rooms.get(&color).unwrap().top_entrance.is_some() {
                        *enter_room = Some(RoomTransitionIn {
                            position: pos,
                            entrance: RoomEntrance::Top,
                            color,
                            timer: 0.,
                        });
                    }
                }

                let right_enter_region = Rect::new(pos.to_f32() + vec2(1., 0.), size2(1., 1.));
                if right_enter_region.contains(player_position) {
                    if rooms.get(&color).unwrap().right_entrance.is_some() {
                        *enter_room = Some(RoomTransitionIn {
                            position: pos,
                            entrance: RoomEntrance::Right,
                            color,
                            timer: 0.,
                        });
                    }
                }

                if enter_room.is_some() {
                    entered = true;
                }
            }
        });

        if entered {
            self.mixer.play(&self.enter_sound, 1.0, false);
            if let Some(handle) = self.run_handle.take() {
                self.mixer.set_looping(&handle, false)
            }
        }
    }

    pub fn draw(&mut self, context: &mut gl::Context) {
        unsafe {
            let bg_color = room_block_colors(self.current_room).background;
            context.clear(
                gl::RenderTarget::Screen,
                [
                    bg_color.0 as f32 / 255.,
                    bg_color.1 as f32 / 255.,
                    bg_color.2 as f32 / 255.,
                    1.0,
                ],
            );
        }

        let player_frame = if self.player.velocity.y > 0. {
            7
        } else if self.player.velocity.y < 0. {
            8
        } else if self.player.animation_timer > 0. {
            1 + (self.player.animation_timer / RUN_ANIMATION_TIME * 6.).floor() as usize
        } else {
            0
        };
        let player_x_flip = if self.player.flip { -1. } else { 1. };

        let mut entity_vertices = Vec::new();

        self.program
            .set_uniform(2, gl::Uniform::Float(1.0))
            .unwrap();

        if let Some(enter_room) = &self.enter_room {
            let player_offset = vec2(0.5, -self.player.collision_rect.min_y());
            let room_entrance = self
                .rooms
                .get(&enter_room.color)
                .unwrap()
                .entrance(enter_room.entrance)
                .unwrap();

            let ratio = enter_room.timer / ENTER_ROOM_TIME;

            // Shrink player and move them to the entrance of the next room
            let player_shrink_start = ENTER_ROOM_TIME * 0.25;
            let player_shrink_time = ENTER_ROOM_TIME * 0.75;
            let shrink_ratio = ((enter_room.timer - player_shrink_start)
                / (player_shrink_time - player_shrink_start))
                .min(1.0)
                .max(0.);
            let player_scale = lerp(shrink_ratio, 1., 1. / TILE_SIZE);

            let entrance_offset = match enter_room.entrance {
                RoomEntrance::Left => vec2(-2.0, 0.0),
                RoomEntrance::Top => vec2(0.0, 2.0),
                RoomEntrance::Right => vec2(2.0, 0.0),
            };
            let outside_entrance_pos = enter_room.position.to_f32()
                + (room_entrance.to_f32().to_vector() + player_offset) / TILE_SIZE
                + entrance_offset / TILE_SIZE;
            let player_pos = if enter_room.timer < player_shrink_time {
                // first move player to just outside the entrance
                self.player.position + (outside_entrance_pos - self.player.position) * shrink_ratio
            } else {
                let r = (enter_room.timer - player_shrink_time)
                    / (ENTER_ROOM_TIME - player_shrink_time);
                let room_entrance_pos = enter_room.position.to_f32()
                    + (room_entrance.to_f32().to_vector() + player_offset) / TILE_SIZE;
                outside_entrance_pos + (room_entrance_pos - outside_entrance_pos) * r
            };
            self.player.sprite.set_transform(
                Transform2D::translation(-7.5, -7.5)
                    .then_scale(1. / TILE_SIZE * player_x_flip, 1. / TILE_SIZE)
                    .then_scale(player_scale, player_scale),
            );
            render_sprite(
                &self.player.sprite,
                player_frame,
                player_pos,
                &mut entity_vertices,
            );

            let room_position = enter_room.position.to_f32().to_vector();

            let camera_bl = enter_room.position.to_f32().to_vector() * ratio;
            let from_camera_tr = point2(ROOM_SIZE.0, ROOM_SIZE.1).to_f32();
            let to_camera_tr = enter_room.position.to_f32() + vec2(1.0, 1.0);
            let camera_tr = from_camera_tr + (to_camera_tr - from_camera_tr) * ratio;
            let camera_scale = ROOM_SIZE.0 as f32 / (camera_tr.x - camera_bl.x);
            let transform = Transform2D::translation(-camera_bl.x, -camera_bl.y)
                .then_scale(camera_scale, camera_scale)
                .then_scale(1.0 / SCREEN_SIZE.0 as f32, 1.0 / SCREEN_SIZE.0 as f32)
                .then_scale(ZOOM_LEVEL, ZOOM_LEVEL)
                .then_scale(TILE_SIZE as f32, TILE_SIZE as f32)
                .then_scale(2., 2.)
                .then_translate(vec2(-1.0, -1.0));
            self.program
                .set_uniform(
                    0,
                    gl::Uniform::Mat3([
                        [transform.m11, transform.m12, 0.0],
                        [transform.m21, transform.m22, 0.0],
                        [transform.m31, transform.m32, 1.0],
                    ]),
                )
                .unwrap();

            unsafe {
                self.vertex_buffer.write(&entity_vertices);

                self.program
                    .set_uniform(
                        1,
                        gl::Uniform::Texture(self.room_textures.get(&self.current_room).unwrap()),
                    )
                    .unwrap();
                self.program
                    .render_vertices(&self.room_vertex_buffer, gl::RenderTarget::Screen)
                    .unwrap();

                self.program
                    .set_uniform(1, gl::Uniform::Texture(&self.atlas_texture))
                    .unwrap();

                self.program
                    .render_vertices(&self.vertex_buffer, gl::RenderTarget::Screen)
                    .unwrap();

                let alpha = ((ratio - 0.5) / 0.5).max(0.0);
                self.program
                    .set_uniform(2, gl::Uniform::Float(alpha))
                    .unwrap();

                let sub_room_transform =
                    Transform2D::scale(1. / ROOM_SIZE.0 as f32, 1. / ROOM_SIZE.1 as f32)
                        .then_translate(room_position)
                        .then(&transform);
                self.program
                    .set_uniform(
                        0,
                        gl::Uniform::Mat3([
                            [sub_room_transform.m11, sub_room_transform.m12, 0.0],
                            [sub_room_transform.m21, sub_room_transform.m22, 0.0],
                            [sub_room_transform.m31, sub_room_transform.m32, 1.0],
                        ]),
                    )
                    .unwrap();

                self.program
                    .set_uniform(
                        1,
                        gl::Uniform::Texture(
                            self.room_textures.get(&enter_room.color).as_ref().unwrap(),
                        ),
                    )
                    .unwrap();
                self.program
                    .render_vertices(&self.room_vertex_buffer, gl::RenderTarget::Screen)
                    .unwrap();
            }
        } else {
            let transform =
                Transform2D::scale(1.0 / SCREEN_SIZE.0 as f32, 1.0 / SCREEN_SIZE.0 as f32)
                    .then_scale(ZOOM_LEVEL, ZOOM_LEVEL)
                    .then_scale(TILE_SIZE as f32, TILE_SIZE as f32)
                    .then_scale(2., 2.)
                    .then_translate(vec2(-1.0, -1.0));
            self.program
                .set_uniform(
                    0,
                    gl::Uniform::Mat3([
                        [transform.m11, transform.m12, 0.0],
                        [transform.m21, transform.m22, 0.0],
                        [transform.m31, transform.m32, 1.0],
                    ]),
                )
                .unwrap();

            self.player.sprite.set_transform(
                Transform2D::translation(-7.5, -7.5)
                    .then_scale(1. / TILE_SIZE * player_x_flip, 1. / TILE_SIZE),
            );
            render_sprite(
                &self.player.sprite,
                player_frame,
                self.player.position,
                &mut entity_vertices,
            );

            unsafe {
                self.vertex_buffer.write(&entity_vertices);
                self.program
                    .set_uniform(1, gl::Uniform::Texture(&self.atlas_texture))
                    .unwrap();
                self.program
                    .render_vertices(&self.vertex_buffer, gl::RenderTarget::Screen)
                    .unwrap();

                self.program
                    .set_uniform(
                        1,
                        gl::Uniform::Texture(
                            self.room_textures.get(&self.current_room).as_ref().unwrap(),
                        ),
                    )
                    .unwrap();
                self.program
                    .render_vertices(&self.room_vertex_buffer, gl::RenderTarget::Screen)
                    .unwrap();
            }
        }

        let transform = Transform2D::scale(1.0 / SCREEN_SIZE.0 as f32, 1.0 / SCREEN_SIZE.0 as f32)
            .then_scale(2., 2.)
            .then_translate(vec2(-1.0, -1.0));
        self.program
            .set_uniform(
                0,
                gl::Uniform::Mat3([
                    [transform.m11, transform.m12, 0.0],
                    [transform.m21, transform.m22, 0.0],
                    [transform.m31, transform.m32, 1.0],
                ]),
            )
            .unwrap();
        let mut ui_vertices = Vec::new();

        render_sprite(
            &self.mute_icon,
            if self.muted { 0 } else { 1 },
            self.mute_icon_rect.min(),
            &mut ui_vertices,
        );
        unsafe {
            self.ui_buffer.write(&ui_vertices);
            self.program
                .set_uniform(1, gl::Uniform::Texture(&self.atlas_texture))
                .unwrap();
            self.program
                .render_vertices(&self.ui_buffer, gl::RenderTarget::Screen)
                .unwrap();
        }
    }
}

struct TileImages {
    // top left
    tl_outer_corner: TextureRect,
    tl_horz: TextureRect,
    tl_vert: TextureRect,
    tl_inner_corner: TextureRect,
    tl_solid: TextureRect,

    // top right
    tr_outer_corner: TextureRect,
    tr_horz: TextureRect,
    tr_vert: TextureRect,
    tr_inner_corner: TextureRect,
    tr_solid: TextureRect,

    // bottom left
    bl_outer_corner: TextureRect,
    bl_horz: TextureRect,
    bl_vert: TextureRect,
    bl_inner_corner: TextureRect,
    bl_solid: TextureRect,

    // bottom right
    br_outer_corner: TextureRect,
    br_horz: TextureRect,
    br_vert: TextureRect,
    br_inner_corner: TextureRect,
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
            tl_inner_corner: to_texture_rect(tl_rect.translate(vec2(45, 0))),
            tl_solid: to_texture_rect(tl_rect.translate(vec2(60, 0))),

            tr_outer_corner: to_texture_rect(tr_rect),
            tr_horz: to_texture_rect(tr_rect.translate(vec2(15, 0))),
            tr_vert: to_texture_rect(tr_rect.translate(vec2(30, 0))),
            tr_inner_corner: to_texture_rect(tr_rect.translate(vec2(45, 0))),
            tr_solid: to_texture_rect(tr_rect.translate(vec2(60, 0))),

            bl_outer_corner: to_texture_rect(bl_rect),
            bl_horz: to_texture_rect(bl_rect.translate(vec2(15, 0))),
            bl_vert: to_texture_rect(bl_rect.translate(vec2(30, 0))),
            bl_inner_corner: to_texture_rect(bl_rect.translate(vec2(45, 0))),
            bl_solid: to_texture_rect(bl_rect.translate(vec2(60, 0))),

            br_outer_corner: to_texture_rect(br_rect),
            br_horz: to_texture_rect(br_rect.translate(vec2(15, 0))),
            br_vert: to_texture_rect(br_rect.translate(vec2(30, 0))),
            br_inner_corner: to_texture_rect(br_rect.translate(vec2(45, 0))),
            br_solid: to_texture_rect(br_rect.translate(vec2(60, 0))),
        }
    }
}

fn build_room_vertex_buffer(
    gl_context: &mut gl::Context,
    room_block_textures: &HashMap<RoomColor, TextureRect>,
    room_color: RoomColor,
    room: &Room,
    tile_images: &TileImages,
) -> gl::VertexBuffer {
    let mut vertices: Vec<Vertex> = Vec::with_capacity(ROOM_CELLS as usize * 4 * 4);
    let get_tile = |x: i32, y: i32| -> Tile {
        if x < 0 || x >= ROOM_SIZE.0 as i32 || y < 0 || y >= ROOM_SIZE.1 as i32 {
            Tile::Solid
        } else {
            let cell = (y as u32 * ROOM_SIZE.0 + x as u32) as usize;
            room.tiles[cell]
        }
    };

    let colors = room_block_colors(room_color);
    let v_color = [
        colors.inner.0 as f32 / 255.,
        colors.inner.1 as f32 / 255.,
        colors.inner.2 as f32 / 255.,
        1.0,
    ];

    let mut room_blocks = Vec::new();
    for (cell, tile) in room.tiles.iter().enumerate() {
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

        let (tl, t, tr, l, r, bl, b, br) = (
            get_tile(x - 1, y + 1) == Tile::Solid,
            get_tile(x, y + 1) == Tile::Solid,
            get_tile(x + 1, y + 1) == Tile::Solid,
            get_tile(x - 1, y) == Tile::Solid,
            get_tile(x + 1, y) == Tile::Solid,
            get_tile(x - 1, y - 1) == Tile::Solid,
            get_tile(x, y - 1) == Tile::Solid,
            get_tile(x + 1, y - 1) == Tile::Solid,
        );

        let rect = Box2D::new(
            point2(x as f32, y as f32),
            point2((x + 1) as f32, (y + 1) as f32),
        );
        let mid = Point2D::new(x as f32 + (8. / TILE_SIZE), y as f32 + (7. / TILE_SIZE));

        // top left rect
        let tl_box = Box2D::new(point2(rect.min.x, mid.y), point2(mid.x, rect.max.y));
        if !tl && t && l {
            graphics::render_quad(tl_box, tile_images.tl_inner_corner, v_color, &mut vertices);
        } else if !l && !t {
            graphics::render_quad(tl_box, tile_images.tl_outer_corner, v_color, &mut vertices);
        } else if l && !t {
            graphics::render_quad(tl_box, tile_images.tl_horz, v_color, &mut vertices);
        } else if !l && t {
            graphics::render_quad(tl_box, tile_images.tl_vert, v_color, &mut vertices);
        } else {
            graphics::render_quad(tl_box, tile_images.tl_solid, v_color, &mut vertices);
        }

        // top right rect
        let tr_box = Box2D::new(point2(mid.x, mid.y), rect.max);
        if !tr && t && r {
            graphics::render_quad(tr_box, tile_images.tr_inner_corner, v_color, &mut vertices);
        } else if !r && !t {
            graphics::render_quad(tr_box, tile_images.tr_outer_corner, v_color, &mut vertices);
        } else if r && !t {
            graphics::render_quad(tr_box, tile_images.tr_horz, v_color, &mut vertices);
        } else if !r && t {
            graphics::render_quad(tr_box, tile_images.tr_vert, v_color, &mut vertices);
        } else {
            graphics::render_quad(tr_box, tile_images.tr_solid, v_color, &mut vertices);
        }

        // bottom left rect
        let bl_box = Box2D::new(rect.min, mid);
        if !bl && b & l {
            graphics::render_quad(bl_box, tile_images.bl_inner_corner, v_color, &mut vertices);
        } else if !l && !b {
            graphics::render_quad(bl_box, tile_images.bl_outer_corner, v_color, &mut vertices);
        } else if l && !b {
            graphics::render_quad(bl_box, tile_images.bl_horz, v_color, &mut vertices);
        } else if !l && b {
            graphics::render_quad(bl_box, tile_images.bl_vert, v_color, &mut vertices);
        } else {
            graphics::render_quad(bl_box, tile_images.bl_solid, v_color, &mut vertices);
        }

        // bottom right rect
        let br_box = Box2D::new(point2(mid.x, rect.min.y), point2(rect.max.x, mid.y));
        if !br && b & r {
            graphics::render_quad(br_box, tile_images.br_inner_corner, v_color, &mut vertices);
        } else if !r && !b {
            graphics::render_quad(br_box, tile_images.br_outer_corner, v_color, &mut vertices);
        } else if r && !b {
            graphics::render_quad(br_box, tile_images.br_horz, v_color, &mut vertices);
        } else if !r && b {
            graphics::render_quad(br_box, tile_images.br_vert, v_color, &mut vertices);
        } else {
            graphics::render_quad(br_box, tile_images.br_solid, v_color, &mut vertices);
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
            [1., 1., 1., 1.],
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
            room.tiles[cell]
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
                && tile_at(tile_x, tile_y - 1)
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
    since_jump: f32,
}

const RUN_ANIMATION_TIME: f32 = 0.5;

struct Player {
    position: Point2D<f32>,
    velocity: Vector2D<f32>,

    since_on_ground: f32,

    sprite: Sprite,
    flip: bool,
    animation_timer: f32,

    collision_rect: Rect<f32>,
    interact_rect: Rect<f32>,
}

impl Player {
    pub fn new(texture: TextureRect, position: Point2D<f32>) -> Player {
        let mut player_sprite = Sprite::new(texture, 9, point2(0., 0.));
        player_sprite.set_transform(
            Transform2D::translation(-7.5, -7.5).then_scale(1. / TILE_SIZE, 1. / TILE_SIZE),
        );

        Player {
            position,
            velocity: vec2(0., 0.),

            since_on_ground: 9999.,

            sprite: player_sprite,
            flip: false,
            animation_timer: -1.,

            collision_rect: Rect::new(
                point2(-3.0 / TILE_SIZE, -7.5 / TILE_SIZE),
                size2(6. / TILE_SIZE, 14. / TILE_SIZE),
            ),
            interact_rect: Rect::new(
                point2(-3.5 / TILE_SIZE, -8.0 / TILE_SIZE),
                size2(7. / TILE_SIZE, 14.5 / TILE_SIZE),
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
    Red,
    Orange,
    Yellow,
    Green,
    Turquoise,
    Aqua,
    Chetwood,
    Blue,
    Purple,
    Magenta,
    Ferrish,
}

impl RoomColor {
    fn hue(&self) -> f32 {
        match self {
            RoomColor::Red => 0.,
            RoomColor::Orange => 26.,
            RoomColor::Yellow => 57.,
            RoomColor::Green => 129.,
            RoomColor::Turquoise => 155.,
            RoomColor::Aqua => 166.,
            RoomColor::Chetwood => 199.,
            RoomColor::Blue => 225.,
            RoomColor::Purple => 255.,
            RoomColor::Magenta => 300.,
            RoomColor::Ferrish => 335.,
        }
    }
}

const ROOM_BLOCK_IMAGE_SIZE: (u32, u32) = (17, 17);

struct RoomBlockColors {
    background: (u8, u8, u8),
    inner: (u8, u8, u8),
    border: (u8, u8, u8),
    outer_border: (u8, u8, u8),
}

impl RoomBlockColors {
    pub fn new(hue: f32) -> RoomBlockColors {
        RoomBlockColors {
            background: LinSrgb::from(Hsv::<palette::encoding::srgb::Srgb, f32>::from_components(
                (hue, 0.21, 0.7),
            ))
            .into_format()
            .into_components(),
            inner: LinSrgb::from(Hsv::<palette::encoding::srgb::Srgb, f32>::from_components(
                (hue, 0.35, 0.6),
            ))
            .into_format()
            .into_components(),
            border: LinSrgb::from(Hsv::<palette::encoding::srgb::Srgb, f32>::from_components(
                (hue, 0.36, 0.47),
            ))
            .into_format()
            .into_components(),
            outer_border: LinSrgb::from(
                Hsv::<palette::encoding::srgb::Srgb, f32>::from_components((hue, 0.42, 0.3)),
            )
            .into_format()
            .into_components(),
        }
    }
}

fn room_block_colors(color: RoomColor) -> RoomBlockColors {
    RoomBlockColors::new(color.hue())
}

const ENTER_ROOM_TIME: f32 = 0.5;

struct RoomTransitionIn {
    position: Point2D<i32>,
    entrance: RoomEntrance,
    color: RoomColor,
    timer: f32,
}

#[derive(Clone, Copy, Debug)]
enum RoomEntrance {
    Left,
    Right,
    Top,
}

struct Room {
    tiles: [Tile; ROOM_CELLS],
    left_entrance: Option<Point2D<i32>>,
    top_entrance: Option<Point2D<i32>>,
    right_entrance: Option<Point2D<i32>>,
}

impl Room {
    pub fn for_each_tile_in_rect(
        &self,
        bound_rect: Rect<f32>,
        mut f: impl FnMut(Point2D<i32>, Tile),
    ) {
        let min_x = (bound_rect.min_x()).floor() as i32;
        let max_x = (bound_rect.max_x()).floor() as i32;
        let min_y = (bound_rect.min_y()).floor() as i32;
        let max_y = (bound_rect.max_y()).floor() as i32;

        for x in min_x..=max_x {
            for y in min_y..=max_y {
                let pos = point2(x, y);
                let tile = if x < 0 || x >= ROOM_SIZE.0 as i32 || y < 0 || y >= ROOM_SIZE.1 as i32 {
                    Tile::Solid
                } else {
                    let cell = (y * ROOM_SIZE.0 as i32 + x) as usize;
                    self.tiles[cell]
                };
                f(pos, tile)
            }
        }
    }

    fn entrance(&self, entrance: RoomEntrance) -> Option<Point2D<i32>> {
        match entrance {
            RoomEntrance::Left => self.left_entrance,
            RoomEntrance::Top => self.top_entrance,
            RoomEntrance::Right => self.right_entrance,
        }
    }
}

fn parse_room(level: &str) -> Room {
    let mut tiles = [Tile::Empty; ROOM_CELLS];

    let mut left_entrance = None;
    let mut top_entrance = None;
    let mut right_entrance = None;

    for (y, line) in level.lines().enumerate() {
        if y >= ROOM_SIZE.1 as usize {
            break;
        }
        for (x, c) in line.chars().enumerate() {
            if x >= ROOM_SIZE.0 as usize {
                break;
            }

            // flip y
            let y = ROOM_SIZE.1 as usize - 1 - y;
            let cell = y * ROOM_SIZE.0 as usize + x;
            let tile = match c {
                ' ' => Tile::Empty,
                '#' => Tile::Solid,
                'R' => Tile::Room(RoomColor::Red),
                'O' => Tile::Room(RoomColor::Orange),
                'Y' => Tile::Room(RoomColor::Yellow),
                'G' => Tile::Room(RoomColor::Green),
                'T' => Tile::Room(RoomColor::Turquoise),
                'A' => Tile::Room(RoomColor::Aqua),
                'C' => Tile::Room(RoomColor::Chetwood),
                'B' => Tile::Room(RoomColor::Blue),
                'P' => Tile::Room(RoomColor::Purple),
                'M' => Tile::Room(RoomColor::Magenta),
                'F' => Tile::Room(RoomColor::Ferrish),
                c @ _ => {
                    panic!("Unrecognized tile identifier '{}'", c);
                }
            };

            let tile_pos = point2(x as i32, y as i32);
            if x == 0 && tile == Tile::Empty {
                left_entrance = Some(tile_pos);
            }
            if x as u32 == ROOM_SIZE.0 - 1 && tile == Tile::Empty {
                right_entrance = Some(tile_pos);
            }
            if y as u32 == ROOM_SIZE.1 - 1 && tile == Tile::Empty {
                top_entrance = Some(tile_pos);
            }
            tiles[cell] = tile;
        }
    }

    Room {
        tiles,
        left_entrance,
        top_entrance,
        right_entrance,
    }
}

fn lerp(x: f32, a: f32, b: f32) -> f32 {
    a + (b - a) * x
}
