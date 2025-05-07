use anyhow::{Context, Result};
use image::io::Reader as ImageReader;
use std::path::Path;

pub struct Texture {
    id: u32,
    width: u32,
    height: u32,
}

impl Texture {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let img = ImageReader::open(path.as_ref())?
            .decode()
            .with_context(|| format!("Failed to decode image at {:?}", path.as_ref()))?
            .to_rgba8();

        let (width, height) = (img.width(), img.height());
        let data = img.into_raw();

        let mut id = 0;
        unsafe {
            gl::GenTextures(1, &mut id);
            gl::BindTexture(gl::TEXTURE_2D, id);
            
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            
            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA as i32,
                width as i32,
                height as i32,
                0,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                data.as_ptr() as *const _,
            );
            
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }

        Ok(Self { id, width, height })
    }

    pub fn bind(&self) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.id);
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.id);
        }
    }
}
