use anyhow::Error;
use euclid::{
    default::{Box2D, Point2D, Rect, Size2D, Transform2D},
    point2, size2,
};
use zerocopy::AsBytes;

use crate::{
    gl,
    texture_atlas::{TextureAtlas, TextureRect},
};

#[repr(C)]
#[derive(Clone, Copy, Debug, AsBytes)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Clone)]
pub struct Sprite {
    frames: Vec<TextureRect>,
    frame_count: u32,
    origin: Point2D<f32>,
    transform: Transform2D<f32>,
}

impl Sprite {
    pub fn new(image: TextureRect, frame_count: u32, origin: Point2D<f32>) -> Self {
        let width = image[2] - image[0];
        let frame_width = width / frame_count;
        let frames = (0..frame_count)
            .map(|i| {
                [
                    image[0] + i * frame_width,
                    image[1],
                    image[0] + (i + 1) * frame_width,
                    image[3],
                ]
            })
            .collect();
        Self {
            frames,
            frame_count,
            origin,
            transform: Transform2D::translation(-origin.x, -origin.y),
        }
    }

    pub fn set_transform(&mut self, t: Transform2D<f32>) {
        self.transform = Transform2D::translation(-self.origin.x, -self.origin.y).then(&t);
    }

    pub fn transform(&self) -> &Transform2D<f32> {
        &self.transform
    }
}

pub unsafe fn load_image(
    image_bytes: &[u8],
    texture_atlas: &mut TextureAtlas,
    texture: &mut gl::Texture,
) -> Result<TextureRect, Error> {
    let image = image::load_from_memory(image_bytes).unwrap().to_rgba();
    let texture_coords = texture_atlas
        .add_texture((image.width(), image.height()))
        .unwrap();
    texture.write(
        texture_coords[0],
        texture_coords[1],
        texture_coords[2] - texture_coords[0],
        texture_coords[3] - texture_coords[1],
        &image.into_raw(),
    );
    Ok(texture_coords)
}

pub unsafe fn load_raw_image(
    bytes: &[u8],
    height: u32,
    width: u32,
    texture_atlas: &mut TextureAtlas,
    texture: &mut gl::Texture,
) -> Result<TextureRect, Error> {
    let texture_coords = texture_atlas.add_texture((width, height)).unwrap();
    texture.write(
        texture_coords[0],
        texture_coords[1],
        texture_coords[2] - texture_coords[0],
        texture_coords[3] - texture_coords[1],
        bytes,
    );
    Ok(texture_coords)
}

pub fn render_sprite(
    sprite: &Sprite,
    frame: usize,
    position: Point2D<f32>,
    color: [f32; 4],
    out: &mut Vec<Vertex>,
) {
    let size = size2(
        (sprite.frames[frame][2] - sprite.frames[frame][0]) as f32,
        (sprite.frames[frame][3] - sprite.frames[frame][1]) as f32,
    );
    let vertex_rect = Rect::new(point2(0., 0.), size);

    let uv_pos = point2(
        sprite.frames[frame][0] as f32 / TEXTURE_ATLAS_SIZE.width as f32,
        sprite.frames[frame][1] as f32 / TEXTURE_ATLAS_SIZE.height as f32,
    );
    let uv_size = size2(
        (sprite.frames[frame][2] - sprite.frames[frame][0]) as f32
            / TEXTURE_ATLAS_SIZE.width as f32,
        (sprite.frames[frame][3] - sprite.frames[frame][1]) as f32
            / TEXTURE_ATLAS_SIZE.height as f32,
    );
    let uv_rect = Rect::new(uv_pos, uv_size);

    let transform = |p: Point2D<f32>| -> [f32; 2] {
        (position + sprite.transform().transform_point(p).to_vector()).to_array()
    };
    out.extend_from_slice(&[
        Vertex {
            position: transform(vertex_rect.min()),
            uv: [uv_rect.min_x(), uv_rect.max_y()],
            color,
        },
        Vertex {
            position: transform(point2(vertex_rect.max_x(), vertex_rect.min_y())),
            uv: [uv_rect.max_x(), uv_rect.max_y()],
            color,
        },
        Vertex {
            position: transform(point2(vertex_rect.min_x(), vertex_rect.max_y())),
            uv: [uv_rect.min_x(), uv_rect.min_y()],
            color,
        },
        Vertex {
            position: transform(point2(vertex_rect.max_x(), vertex_rect.min_y())),
            uv: [uv_rect.max_x(), uv_rect.max_y()],
            color,
        },
        Vertex {
            position: transform(vertex_rect.max()),
            uv: [uv_rect.max_x(), uv_rect.min_y()],
            color,
        },
        Vertex {
            position: transform(point2(vertex_rect.min_x(), vertex_rect.max_y())),
            uv: [uv_rect.min_x(), uv_rect.min_y()],
            color,
        },
    ]);
}

pub fn render_quad(
    rect: Box2D<f32>,
    tex_coords: TextureRect,
    color: [f32; 4],
    out: &mut Vec<Vertex>,
) {
    let uv_pos = point2(
        tex_coords[0] as f32 / TEXTURE_ATLAS_SIZE.width as f32,
        tex_coords[1] as f32 / TEXTURE_ATLAS_SIZE.height as f32,
    );
    let uv_size = size2(
        (tex_coords[2] - tex_coords[0]) as f32 / TEXTURE_ATLAS_SIZE.width as f32,
        (tex_coords[3] - tex_coords[1]) as f32 / TEXTURE_ATLAS_SIZE.height as f32,
    );
    let uv_rect = Rect::new(uv_pos, uv_size);

    out.extend_from_slice(&[
        Vertex {
            position: rect.min.to_array(),
            uv: [uv_rect.min_x(), uv_rect.max_y()],
            color,
        },
        Vertex {
            position: [rect.max.x, rect.min.y],
            uv: [uv_rect.max_x(), uv_rect.max_y()],
            color,
        },
        Vertex {
            position: [rect.min.x, rect.max.y],
            uv: [uv_rect.min_x(), uv_rect.min_y()],
            color,
        },
        Vertex {
            position: [rect.max.x, rect.min.y],
            uv: [uv_rect.max_x(), uv_rect.max_y()],
            color,
        },
        Vertex {
            position: rect.max.to_array(),
            uv: [uv_rect.max_x(), uv_rect.min_y()],
            color,
        },
        Vertex {
            position: [rect.min.x, rect.max.y],
            uv: [uv_rect.min_x(), uv_rect.min_y()],
            color,
        },
    ]);
}

pub const TEXTURE_ATLAS_SIZE: Size2D<u32> = Size2D {
    width: 1024,
    height: 1024,
    _unit: std::marker::PhantomData::<euclid::UnknownUnit>,
};
