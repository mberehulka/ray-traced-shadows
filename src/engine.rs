use winit::{event_loop::EventLoop, window::{WindowBuilder, Window}, dpi::PhysicalSize};
use pixels::{Pixels, SurfaceTexture};
use math::{Vec3, Quaternion};

use crate::{object::Object, camera::Camera, dir_light::DirectionalLight, color};

pub struct Engine {
    pub buff_w4: i32,

    pub width: u32,
    pub height: u32,
    pub pixels: Pixels,
    
    pub window: Window,
    pub objects: Vec<Object>,
    pub camera: Camera,
    pub dir_light: DirectionalLight
}

impl Engine {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let window = WindowBuilder::new()
            .with_resizable(false)
            .with_inner_size(PhysicalSize {
                width: 800,
                height: 600
            })
            .build(&event_loop).unwrap();
        let width = window.inner_size().width;
        let height = window.inner_size().height;
        let pixels = Pixels::new(width, height, SurfaceTexture::new(width, height, &window)).unwrap();
        Self {
            buff_w4: window.inner_size().width as i32 * 4,

            width: window.inner_size().width,
            height: window.inner_size().height,
            pixels,
            
            window,
            objects: vec![],
            camera: Camera::new(),
            dir_light: DirectionalLight::default()
        }
    }
    pub fn clear(
        &self,
        pixels: &mut [u8]
    ) {
        let l = pixels.len();
        let mut i = 0;
        while i < l {
            pixels[i] = 0;
            pixels[i+1] = 0;
            pixels[i+2] = 0;
            pixels[i+3] = 255;
            i += 4;
        }
    }
    pub fn rotate_objects(&mut self) {
        for object in self.objects.iter_mut() {
            object.transform.rotation = object.transform.rotation * Quaternion::from_angle_y(0.001);
        }
    }
    pub fn update(&mut self) {
        self.rotate_objects();
        
        let mut pixels = self.pixels.frame().to_vec();
        let mut zbuffer = vec![f32::MAX;pixels.len()];
        self.clear(&mut pixels);
        self.camera.update(self.width, self.height);
        
        for object in self.objects.iter() {
            self.draw(&mut pixels, &mut zbuffer, object)
        }

        self.pixels.frame_mut().copy_from_slice(&pixels);
        self.pixels.render().unwrap()
    }
    pub fn draw(
        &self,
        pixels: &mut [u8],
        zbuffer: &mut [f32],
        object: &Object
    ) {
        for [a, b, c] in object.vertices.iter() {
            let ap = object.transform * a.position;
            let bp = object.transform * b.position;
            let cp = object.transform * c.position;
            let an = (object.transform.rotation * a.normal).normalized();
            
            if an.dot(self.camera.position - ap) <= 0. { continue }

            let dp = an.dot(self.dir_light.direction);
            
            self.project_triangle(
                pixels,
                zbuffer,
                self.camera.mat * ap,
                self.camera.mat * bp,
                self.camera.mat * cp,
                color::from_f32(1. - dp * 0.75),
                // [255;3]
            )
        }
    }
    #[inline(always)]
    pub fn project_triangle(
        &self,
        pixels: &mut [u8],
        zbuffer: &mut [f32],
        a: Vec3,
        b: Vec3,
        c: Vec3,
        color: [u8;3]
    ) {
        if a.z <= 0. || b.z <= 0. || c.z <= 0. { return }
        self.raster_triangle(
            pixels,
            zbuffer,
            ((a.x + 1.) * 0.5 * self.width as f32) as i32,
            ((a.y + 1.) * 0.5 * self.height as f32) as i32,
            a.z,
            ((b.x + 1.) * 0.5 * self.width as f32) as i32,
            ((b.y + 1.) * 0.5 * self.height as f32) as i32,
            b.z,
            ((c.x + 1.) * 0.5 * self.width as f32) as i32,
            ((c.y + 1.) * 0.5 * self.height as f32) as i32,
            c.z,
            color
        )
    }
    #[inline(always)]
    pub fn raster_triangle(
        &self,
        pixels: &mut [u8],
        zbuffer: &mut [f32],
        ax: i32, ay: i32, az: f32,
        bx: i32, by: i32, bz: f32,
        cx: i32, cy: i32, cz: f32,
        color: [u8;3]
    ) {
        let max_width = self.width as i32 - 1;
        let max_height = self.height as i32 - 1;
        
        let minx = max_width.min(ax).max(0).min(bx).max(0).min(cx).max(0);
        let mut miny = max_height.min(ay).max(0).min(by).max(0).min(cy).max(0);
        let maxx = 0.max(ax).min(max_width).max(bx).min(max_width).max(cx).min(max_width);
        let maxy = 0.max(ay).min(max_height).max(by).min(max_height).max(cy).min(max_height);

        let l1x = cx as f32 - ax as f32;
        let l1y = bx as f32 - ax as f32;
        let mut l1z;
        let l2x = cy as f32 - ay as f32;
        let l2y = by as f32 - ay as f32;
        let mut l2z;
        
        let mut ux; let mut uy;
        let uz = (l1x * l2y) - (l1y * l2x);
        if uz.abs() < 1. { return }

        let mut i = (miny * self.width as i32) as usize * 4;
        let mut x;
        let line_width = (self.width as i32 - maxx - 1) as usize * 4;
        let line_offset = minx as usize * 4;
        let mut btx; let mut bty; let mut btz;
        let mut z;
        
        while miny <= maxy {
            l2z = ay as f32 - miny as f32;
            x = minx;
            i += line_offset;
            while x <= maxx {
                l1z = ax as f32 - x as f32;
                ux = (l1y * l2z) - (l1z * l2y);
                uy = (l1z * l2x) - (l1x * l2z);
                btx = 1.-(ux+uy)/uz;
                bty = uy/uz;
                btz = ux/uz;
                if btx < 0. || bty < 0. || btz < 0. {
                    x += 1;
                    i += 4;
                    continue
                }
                z = (az * btx) + (bz * bty) + (cz * btz);
                if zbuffer[i] >= z {
                    pixels[i    ] = color[0];
                    pixels[i + 1] = color[2];
                    pixels[i + 2] = color[1];
                    zbuffer[i] = z;
                }
                x += 1;
                i += 4
            }
            miny += 1;
            i += line_width
        }
    }
}