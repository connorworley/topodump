use std::cmp::min;
use std::io::{self, BufRead, Cursor, Read};

use image::{ImageFormat, RgbaImage, Rgba, imageops};
use image::io::Reader as ImageReader;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
struct TpqHeader {
  version:       u32,
  w_long:        f64,
  n_lat:         f64,
  e_long:        f64,
  s_lat:         f64,
  topo:          String,
  quad_name:     String,
  state_name:    String,
  source:        String,
  year1:         String,
  year2:         String,
  contour:       String,
  extension:     String,
  color_depth:   u32,
  long_count:    u32,
  lat_count:     u32,
  maplet_width:  u32,
  maplet_height: u32,
}

fn read_tpq_u32(input: &mut impl Read) -> Result<u32> {
   let mut buf: [u8; 4] = Default::default();
   input.read_exact(&mut buf)?;
   return Ok(u32::from_le_bytes(buf));
}

fn read_tpq_f64(input: &mut impl Read) -> Result<f64> {
   let mut buf: [u8; 8] = Default::default();
   input.read_exact(&mut buf)?;
   return Ok(f64::from_le_bytes(buf));
}

fn read_tpq_string(input: &mut impl Read, size: usize) -> Result<String> {
   let mut buf = vec![0u8; size+1];
   input.read_exact(&mut buf[..size])?;
   let mut chars = Vec::<u8>::new();
   let chars_read = Cursor::new(buf).read_until(0, &mut chars)?;
   return Ok(String::from_utf8_lossy(&chars[..min(chars_read-1, size)]).to_string());
}

fn read_tpq_header(input: &mut impl Read) -> Result<TpqHeader> {
  return Ok(TpqHeader{
       version:       read_tpq_u32(input)?,
       w_long:        read_tpq_f64(input)?,
       n_lat:         read_tpq_f64(input)?,
       e_long:        read_tpq_f64(input)?,
       s_lat:         read_tpq_f64(input)?,
       topo:          read_tpq_string(input, 220)?,
       quad_name:     read_tpq_string(input, 128)?,
       state_name:    read_tpq_string(input, 32)?,
       source:        read_tpq_string(input, 32)?,
       year1:         read_tpq_string(input, 4)?,
       year2:         read_tpq_string(input, 4)?,
       contour:       read_tpq_string(input, 24)?,
       extension:     read_tpq_string(input, 4)?,
       color_depth:   read_tpq_u32(input)?,
                      // Skip unknown u32
       long_count:    { read_tpq_u32(input)?; read_tpq_u32(input)? },
       lat_count:     read_tpq_u32(input)?,
       maplet_width:  read_tpq_u32(input)?,
       maplet_height: read_tpq_u32(input)?,
   });
}

struct WorldFile {
  scale_x: i32,
  skew_y: i32,
  skew_x: i32,
  scale_y: i32,
  anchor_x: i32,
  anchor_y: i32,
}


fn main() -> Result<()> {
   let mut tpq_data = Vec::<u8>::new();
   io::stdin().read_to_end(&mut tpq_data)?;
   let mut cursor = Cursor::new(&tpq_data);

   let header = read_tpq_header(&mut cursor)?;
   println!("{:#?}", header);

   cursor.set_position(1024);

   let mut collage_img = RgbaImage::from_pixel(
     header.long_count * header.maplet_width,
     header.lat_count * header.maplet_height,
     Rgba([255, 255, 255, 255]),
   );

   for i in 0..header.lat_count {
     for j in 0..header.long_count {
       let maplet_offset = read_tpq_u32(&mut cursor)? as usize;
       let maplet_cursor = Cursor::new(&tpq_data[maplet_offset..]);
       let img = ImageReader::with_format(maplet_cursor, ImageFormat::Jpeg).decode()?;
       imageops::overlay(&mut collage_img, &img, j * header.maplet_width, i * header.maplet_height);
     }
   }

   collage_img.save("collage.jpg")?;

   Ok(())
}