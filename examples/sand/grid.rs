use glam::{IVec2, Vec2};
use glass::{pipelines::QuadPipeline, texture::Texture};
use image::RgbaImage;
use wgpu::{
    BindGroup, Device, Extent3d, ImageCopyTexture, ImageDataLayout, Origin3d, Queue, Sampler,
    TextureAspect, TextureFormat, TextureUsages,
};

use crate::sand::{Sand, SandType};

pub struct Grid {
    pub data: Vec<Sand>,
    pub rgba: RgbaImage,
    pub texture: Texture,
    pub grid_bind_group: BindGroup,
    pub width: u32,
    pub height: u32,
    changed: bool,
}

impl Grid {
    pub fn new(
        device: &Device,
        quad: &QuadPipeline,
        sampler: &Sampler,
        width: u32,
        height: u32,
    ) -> Grid {
        let data = vec![Sand::empty(); (width * height) as usize];
        let rgba = RgbaImage::new(width, height);
        let texture = Texture::empty(
            device,
            "grid",
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            1,
            TextureFormat::Rgba8UnormSrgb,
            TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        );
        let grid_bind_group = quad.create_bind_group(device, &texture.views[0], sampler);
        Grid {
            data,
            rgba,
            texture,
            grid_bind_group,
            width,
            height,
            changed: false,
        }
    }

    fn index(&self, x: i32, y: i32) -> usize {
        ((self.height as i32 - y - 1) * self.width as i32 + x) as usize
    }

    pub fn draw_sand_radius(&mut self, x: i32, y: i32, sand: SandType, radius: f32) {
        let y_start = y - radius as i32;
        let y_end = y + radius as i32;
        let x_start = x - radius as i32;
        let x_end = x + radius as i32;
        let center = Vec2::new(x as f32, y as f32);
        for pixel_y in y_start..=y_end {
            for pixel_x in x_start..=x_end {
                let pos = IVec2::new(pixel_x, pixel_y);
                let pos_f32 = pos.as_vec2();
                if pos_f32.distance(center).round() < radius.round() {
                    self.draw_sand(pos.x, pos.y, Sand::from_type(sand, pos_f32));
                }
            }
        }
    }

    pub fn draw_sand(&mut self, x: i32, y: i32, sand: Sand) {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            let index = self.index(x, y);
            self.data[index] = sand;
            self.rgba.put_pixel(
                x as u32,
                self.height - y as u32 - 1,
                [sand.color[0], sand.color[1], sand.color[2], 255].into(),
            );

            self.changed = true;
        }
    }

    pub fn simulate(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                // Current
                let curr_index = self.index(x as i32, y as i32);
                let curr = self.data[curr_index];
                let curr_sand = curr.sand;
                // Below
                if !curr_sand.is_empty() && y as i32 > 0 {
                    let below_index = self.index(x as i32, y as i32 - 1);
                    let below_sand = self.data[below_index].sand;
                    let below = self.data[below_index];
                    let swap_below =
                        below_sand.is_empty() || (!curr_sand.is_water() && below_sand.is_water());
                    if swap_below {
                        self.draw_sand(x as i32, y as i32, below);
                        self.draw_sand(x as i32, y as i32 - 1, curr);
                    } else {
                        let p = rand::random::<f32>();
                        let mut is_swap = false;
                        if p > 0.5 && x as i32 > 0 {
                            let left_diag_index = self.index(x as i32 - 1, y as i32 - 1);
                            let left_diag_sand = self.data[left_diag_index].sand;
                            let left = self.data[left_diag_index];
                            let swap_left_diag = left_diag_sand.is_empty()
                                || (!curr_sand.is_water() && left_diag_sand.is_water());
                            if swap_left_diag {
                                self.draw_sand(x as i32, y as i32, left);
                                self.draw_sand(x as i32 - 1, y as i32 - 1, curr);
                                is_swap = true;
                            }
                        } else if x as i32 + 1 < self.width as i32 {
                            let right_diag_index = self.index(x as i32 + 1, y as i32 - 1);
                            let right_diag_sand = self.data[right_diag_index].sand;
                            let swap_right_diag = right_diag_sand.is_empty()
                                || (!curr_sand.is_water() && right_diag_sand.is_water());
                            let right = self.data[right_diag_index];
                            if swap_right_diag {
                                self.draw_sand(x as i32, y as i32, right);
                                self.draw_sand(x as i32 + 1, y as i32 - 1, curr);
                                is_swap = true;
                            }
                        }
                        if !is_swap && curr_sand.is_water() {
                            let p = rand::random::<f32>();
                            if p > 0.5 && x as i32 > 0 {
                                let left_index = self.index(x as i32 - 1, y as i32);
                                let left_sand = self.data[left_index].sand;
                                let left = self.data[left_index];
                                let swap_left = left_sand.is_empty();
                                if swap_left {
                                    self.draw_sand(x as i32, y as i32, left);
                                    self.draw_sand(x as i32 - 1, y as i32, curr);
                                }
                            } else if x as i32 + 1 < self.width as i32 {
                                let right_index = self.index(x as i32 + 1, y as i32);
                                let right_sand = self.data[right_index].sand;
                                let right = self.data[right_index];
                                let swap_right = right_sand.is_empty();
                                if swap_right {
                                    self.draw_sand(x as i32, y as i32, right);
                                    self.draw_sand(x as i32 + 1, y as i32, curr);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn update_texture(&mut self, queue: &Queue) {
        if self.changed {
            queue.write_texture(
                ImageCopyTexture {
                    texture: &self.texture.texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                &self.rgba,
                ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * self.width),
                    rows_per_image: None,
                },
                self.texture.texture.size(),
            );
            self.changed = false;
        }
    }
}
