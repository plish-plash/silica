use std::io::{BufRead, Error as IoError, ErrorKind, Seek};

use png::*;

pub struct Image {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Image {
    pub fn read<R: BufRead + Seek>(reader: R) -> Result<Self, DecodingError> {
        let mut decoder = Decoder::new(reader);
        decoder.set_transformations(Transformations::ALPHA);
        let mut image_reader = decoder.read_info()?;
        let mut data = vec![0; image_reader.output_buffer_size().unwrap()];
        let info = image_reader.next_frame(&mut data)?;
        data.truncate(info.buffer_size());
        assert_eq!(info.bit_depth, BitDepth::Eight);
        match info.color_type {
            ColorType::Rgba => {}
            ColorType::GrayscaleAlpha => {
                data = data.chunks_exact(2).flat_map(|x| [x[0], x[0], x[0], x[1]]).collect();
            }
            _ => {
                return Err(DecodingError::IoError(IoError::new(
                    ErrorKind::Unsupported,
                    format!("unsupported color type {:?}", info.color_type),
                )));
            }
        }
        Ok(Image {
            width: info.width,
            height: info.height,
            data,
        })
    }
}
