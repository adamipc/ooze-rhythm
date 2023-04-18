use glium::Surface;
use std::borrow::Cow;
use std::collections::VecDeque;
use std::vec::Vec;

// Container that holds image data as vector of (u8, u8, u8, u8).
// This is used to take data from PixelBuffer and move it to another thread
// with minimum conversions done on main thread.
pub struct RGBAImageData {
    pub data: Vec<(u8, u8, u8, u8)>,
    pub width: u32,
    pub height: u32,
}

impl glium::texture::Texture2dDataSink<(u8, u8, u8, u8)> for RGBAImageData {
    fn from_raw(data: Cow<'_, [(u8, u8, u8, u8)]>, width: u32, height: u32) -> Self {
        RGBAImageData {
            data: data.into_owned(),
            width,
            height,
        }
    }
}

pub fn save_screenshot(image_data: RGBAImageData, image_path: String) {
    let pixels = {
        let mut v = Vec::with_capacity(image_data.data.len() * 4);
        for (a, b, c, d) in image_data.data {
            v.push(a);
            v.push(b);
            v.push(c);
            v.push(d);
        }
        v
    };

    let image_buffer =
        image::ImageBuffer::from_raw(image_data.width, image_data.height, pixels).unwrap();

    let image = image::DynamicImage::ImageRgba8(image_buffer).flipv();
    image.save(image_path).unwrap();
}

struct AsyncScreenshotTask {
    pub target_frame: u64,
    pub pixel_buffer: glium::texture::pixel_buffer::PixelBuffer<(u8, u8, u8, u8)>,
}

impl AsyncScreenshotTask {
    fn new(facade: &dyn glium::backend::Facade, target_frame: u64) -> Self {
        // Get information about current framebuffer
        let dimensions = facade.get_context().get_framebuffer_dimensions();
        let rect = glium::Rect {
            left: 0,
            bottom: 0,
            width: dimensions.0,
            height: dimensions.1,
        };
        let blit_target = glium::BlitTarget {
            left: 0,
            bottom: 0,
            width: dimensions.0 as i32,
            height: dimensions.1 as i32,
        };

        // Create temporary texture and blit the front buffer to it
        let texture = glium::texture::Texture2d::empty(facade, dimensions.0, dimensions.1).unwrap();
        let framebuffer = glium::framebuffer::SimpleFrameBuffer::new(facade, &texture).unwrap();
        framebuffer.blit_from_frame(
            &rect,
            &blit_target,
            glium::uniforms::MagnifySamplerFilter::Nearest,
        );

        // Read the texture into new pixel buffer
        let pixel_buffer = texture.read_to_pixel_buffer();

        AsyncScreenshotTask {
            target_frame,
            pixel_buffer,
        }
    }

    fn read_image_data(self) -> RGBAImageData {
        self.pixel_buffer.read_as_texture_2d().unwrap()
    }
}

pub struct ScreenshotIterator<'a>(&'a mut AsyncScreenshotTaker);

impl<'a> Iterator for ScreenshotIterator<'a> {
    type Item = RGBAImageData;

    fn next(&mut self) -> Option<RGBAImageData> {
        if self
            .0
            .screenshot_tasks
            .front()
            .map(|task| task.target_frame)
            == Some(self.0.frame)
        {
            let task = self.0.screenshot_tasks.pop_front().unwrap();
            Some(task.read_image_data())
        } else {
            None
        }
    }
}

pub struct AsyncScreenshotTaker {
    screenshot_delay: u64,
    frame: u64,
    screenshot_tasks: VecDeque<AsyncScreenshotTask>,
}

impl AsyncScreenshotTaker {
    pub fn new(screenshot_delay: u64) -> Self {
        AsyncScreenshotTaker {
            screenshot_delay,
            frame: 0,
            screenshot_tasks: VecDeque::new(),
        }
    }

    pub fn next_frame(&mut self) {
        self.frame += 1;
    }

    pub fn pickup_screenshots(&mut self) -> ScreenshotIterator<'_> {
        ScreenshotIterator(self)
    }

    pub fn take_screenshot(&mut self, facade: &dyn glium::backend::Facade) {
        self.screenshot_tasks.push_back(AsyncScreenshotTask::new(
            facade,
            self.frame + self.screenshot_delay,
        ));
    }
}
