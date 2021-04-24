use anyhow::Error;
use euclid::{
    default::{Point2D, Rect, Size2D, Transform2D},
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
    position: [f32; 2],
    uv: [f32; 2],
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
            transform: Transform2D::create_translation(-origin.x, -origin.y),
        }
    }

    pub fn set_transform(&mut self, t: Transform2D<f32>) {
        self.transform =
            Transform2D::create_translation(-self.origin.x, -self.origin.y).post_transform(&t);
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

fn render_line(
    segment: &mut Sprite,
    end: &mut Sprite,
    start_point: Point2D<f32>,
    end_point: Point2D<f32>,
    out: &mut Vec<Vertex>,
) {
    let angle = (end_point - start_point).angle_from_x_axis();

    segment.set_transform(
        Transform2D::create_scale(
            (end_point - start_point).length()
                / (segment.frames[0][2] as f32 - segment.frames[0][0] as f32),
            1.0,
        )
        .post_rotate(-angle),
    );
    end.set_transform(Transform2D::create_rotation(-angle));
    render_sprite(segment, 0, start_point, out);
    render_sprite(end, 0, end_point, out);
}

pub fn render_sprite(sprite: &Sprite, frame: usize, position: Point2D<f32>, out: &mut Vec<Vertex>) {
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
        },
        Vertex {
            position: transform(point2(vertex_rect.max_x(), vertex_rect.min_y())),
            uv: [uv_rect.max_x(), uv_rect.max_y()],
        },
        Vertex {
            position: transform(point2(vertex_rect.min_x(), vertex_rect.max_y())),
            uv: [uv_rect.min_x(), uv_rect.min_y()],
        },
        Vertex {
            position: transform(point2(vertex_rect.max_x(), vertex_rect.min_y())),
            uv: [uv_rect.max_x(), uv_rect.max_y()],
        },
        Vertex {
            position: transform(vertex_rect.max()),
            uv: [uv_rect.max_x(), uv_rect.min_y()],
        },
        Vertex {
            position: transform(point2(vertex_rect.min_x(), vertex_rect.max_y())),
            uv: [uv_rect.min_x(), uv_rect.min_y()],
        },
    ]);
}

pub fn render_quad(position: Point2D<f32>, tex_coords: TextureRect, out: &mut Vec<Vertex>) {
    let size = size2(
        (tex_coords[2] - tex_coords[0]) as f32,
        (tex_coords[3] - tex_coords[1]) as f32,
    );
    let vertex_rect = Rect::new(position, size);

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
            position: vertex_rect.min().to_array(),
            uv: [uv_rect.min_x(), uv_rect.max_y()],
        },
        Vertex {
            position: [vertex_rect.max_x(), vertex_rect.min_y()],
            uv: [uv_rect.max_x(), uv_rect.max_y()],
        },
        Vertex {
            position: [vertex_rect.min_x(), vertex_rect.max_y()],
            uv: [uv_rect.min_x(), uv_rect.min_y()],
        },
        Vertex {
            position: [vertex_rect.max_x(), vertex_rect.min_y()],
            uv: [uv_rect.max_x(), uv_rect.max_y()],
        },
        Vertex {
            position: vertex_rect.max().to_array(),
            uv: [uv_rect.max_x(), uv_rect.min_y()],
        },
        Vertex {
            position: [vertex_rect.min_x(), vertex_rect.max_y()],
            uv: [uv_rect.min_x(), uv_rect.min_y()],
        },
    ]);
}

pub const TEXTURE_ATLAS_SIZE: Size2D<u32> = Size2D {
    width: 1024,
    height: 1024,
    _unit: std::marker::PhantomData::<euclid::UnknownUnit>,
};
