use std::cmp::min;
use std::io::{self, BufRead, Cursor, Read};
use std::path::Path;

use image::io::Reader as ImageReader;
use image::{imageops, ImageFormat, Rgba, RgbaImage};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
struct TpqHeader {
    version: u32,
    w_long: f64,
    n_lat: f64,
    e_long: f64,
    s_lat: f64,
    topo: String,
    quad_name: String,
    state_name: String,
    source: String,
    year1: String,
    year2: String,
    contour: String,
    extension: String,
    color_depth: u32,
    long_count: u32,
    lat_count: u32,
    maplet_width: u32,
    maplet_height: u32,
}

fn read_tpq_u32(input: &mut impl Read) -> Result<u32> {
    let mut buf: [u8; 4] = Default::default();
    input.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_tpq_f64(input: &mut impl Read) -> Result<f64> {
    let mut buf: [u8; 8] = Default::default();
    input.read_exact(&mut buf)?;
    Ok(f64::from_le_bytes(buf))
}

fn read_tpq_string(input: &mut impl Read, size: usize) -> Result<String> {
    let mut buf = vec![0u8; size + 1];
    input.read_exact(&mut buf[..size])?;
    let mut chars = Vec::<u8>::new();
    let chars_read = Cursor::new(buf).read_until(0, &mut chars)?;
    Ok(String::from_utf8_lossy(&chars[..min(chars_read - 1, size)]).to_string())
}

fn read_tpq_header(input: &mut impl Read) -> Result<TpqHeader> {
    Ok(TpqHeader {
        version: read_tpq_u32(input)?,
        w_long: read_tpq_f64(input)?,
        n_lat: read_tpq_f64(input)?,
        e_long: read_tpq_f64(input)?,
        s_lat: read_tpq_f64(input)?,
        topo: read_tpq_string(input, 220)?,
        quad_name: read_tpq_string(input, 128)?,
        state_name: read_tpq_string(input, 32)?,
        source: read_tpq_string(input, 32)?,
        year1: read_tpq_string(input, 4)?,
        year2: read_tpq_string(input, 4)?,
        contour: read_tpq_string(input, 24)?,
        extension: read_tpq_string(input, 4)?,
        color_depth: read_tpq_u32(input)?,
        // Skip unknown u32
        long_count: {
            read_tpq_u32(input)?;
            read_tpq_u32(input)?
        },
        lat_count: read_tpq_u32(input)?,
        maplet_width: read_tpq_u32(input)?,
        maplet_height: read_tpq_u32(input)?,
    })
}

fn main() -> Result<()> {
    let mut tpq_data = Vec::<u8>::new();
    io::stdin().read_to_end(&mut tpq_data)?;
    let mut cursor = Cursor::new(&tpq_data);

    let header = read_tpq_header(&mut cursor)?;
    eprintln!("{:#?}", header);

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
            imageops::overlay(
                &mut collage_img,
                &img,
                j * header.maplet_width,
                i * header.maplet_height,
            );
        }
    }

    let filename = &format!("map_{}_{}.tif", header.w_long, header.n_lat);
    collage_img.save(filename)?;

    let dataset = gdal::Dataset::open_ex(Path::new(filename), Some(1), None, None, None)?;

    let spatial_ref = r#"GEODCRS["NAD 27",
    DATUM["North American Datum of 1927",
        ELLIPSOID["NAD 27", 6378206.4, 294.978698214, LENGTHUNIT["metre", 1]]],
    CS[ellipsoidal, 2],
        AXIS["Latitude (lat)", north, ORDER[1]],
        AXIS["Longitude (lon)", east, ORDER[2]],
        ANGLEUNIT["degree", 0.0174532925199433]]"#;

    dataset.set_spatial_ref(&gdal::spatial_ref::SpatialRef::from_wkt(&spatial_ref)?)?;
    dataset.set_geo_transform(&[
        header.w_long,
        (header.e_long - header.w_long) / collage_img.width() as f64,
        0.0,
        header.n_lat,
        0.0,
        -(header.n_lat - header.s_lat) / collage_img.height() as f64,
    ])?;

    Ok(())
}
