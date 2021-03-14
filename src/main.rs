use std::cmp::min;
use std::fs::File;
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
    return Ok(u32::from_le_bytes(buf));
}

fn read_tpq_f64(input: &mut impl Read) -> Result<f64> {
    let mut buf: [u8; 8] = Default::default();
    input.read_exact(&mut buf)?;
    return Ok(f64::from_le_bytes(buf));
}

fn read_tpq_string(input: &mut impl Read, size: usize) -> Result<String> {
    let mut buf = vec![0u8; size + 1];
    input.read_exact(&mut buf[..size])?;
    let mut chars = Vec::<u8>::new();
    let chars_read = Cursor::new(buf).read_until(0, &mut chars)?;
    return Ok(String::from_utf8_lossy(&chars[..min(chars_read - 1, size)]).to_string());
}

fn read_tpq_header(input: &mut impl Read) -> Result<TpqHeader> {
    return Ok(TpqHeader {
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
    });
}

fn lat_long_to_utm_nad27(lat: f64, long: f64) -> (f64, f64, u32) {
    // https://en.wikipedia.org/wiki/Universal_Transverse_Mercator_coordinate_system#Simplified_formulae
    // https://en.wikipedia.org/wiki/North_American_Datum#North_American_Datum_of_1927
    let semimajor_axis = 6378206.4;
    let flattening: f64 = 1.0 / 294.978698214;

    let n = flattening / (2.0 - flattening);
    let A = semimajor_axis / (1.0 + n) * (1.0 + n.powi(2) / 4.0 + n.powi(4) / 64.0);
    let alpha_1 = 1.0 / 2.0 * n - 2.0 / 3.0 * n.powi(2) + 5.0 / 16.0 * n.powi(3);
    let alpha_2 = 13.0 / 48.0 * n.powi(2) - 3.0 / 5.0 * n.powi(3);
    let alpha_3 = 61.0 / 240.0 * n.powi(3);
    // let beta_1 = 1.0 / 2.0 * n - 2.0 / 3.0 * n.powi(2) + 37.0 / 96.0 * n.powi(3);
    // let beta_2 = 1.0 / 48.0 * n.powi(2) + 1.0 / 15.0 * n.powi(3);
    // let beta_3 = 17.0 / 480.0 * n.powi(3);
    // let delta_1 = 2.0 * n - 2.0 / 3.0 * n.powi(2) - 2.0 * n.powi(3);
    // let delta_2 = 7.0 / 3.0 * n.powi(2) - 8.0 / 5.0 * n.powi(3);
    // let delta_3 = 56.0 / 15.0 * n.powi(3);

    let zone = ((long + 186.0) / 6.0) as u32;
    let center_meridian_rad = (-183.0 + zone as f64 * 6.0).to_radians();

    let lat_rad = lat.to_radians();
    let long_rad = long.to_radians();

    let t = (lat_rad.sin().atanh() - 2.0 * n.sqrt() / (1.0 + n) * (2.0 * n.sqrt() / (1.0 + n) * lat_rad.sin()).atanh()).sinh();
    let xi_prime = (t / (long_rad - center_meridian_rad).cos()).atan();
    let eta_prime = ((long_rad - center_meridian_rad).sin() / (1.0 + t.powi(2)).sqrt()).atanh();
    // let sigma = 1.0 + (
    //     2.0 * 1.0 * alpha_1 * (2.0 * 1.0 * xi_prime).cos() * (2.0 * 1.0 * eta_prime).cosh() +
    //     2.0 * 2.0 * alpha_2 * (2.0 * 2.0 * xi_prime).cos() * (2.0 * 2.0 * eta_prime).cosh() +
    //     2.0 * 3.0 * alpha_3 * (2.0 * 3.0 * xi_prime).cos() * (2.0 * 3.0 * eta_prime).cosh()
    // );
    // let tau = (
    //     2.0 * 1.0 * alpha_1 * (2.0 * 1.0 * xi_prime).sin() * (2.0 * 1.0 * eta_prime).sinh() +
    //     2.0 * 2.0 * alpha_2 * (2.0 * 2.0 * xi_prime).sin() * (2.0 * 2.0 * eta_prime).sinh() +
    //     2.0 * 3.0 * alpha_3 * (2.0 * 3.0 * xi_prime).sin() * (2.0 * 3.0 * eta_prime).sinh()
    // );

    let easting = 500000.0 + 0.9996 * A * (eta_prime + (
        alpha_1 * (2.0 * 1.0 * xi_prime).cos() * (2.0 * 1.0 * eta_prime).sinh() +
        alpha_2 * (2.0 * 2.0 * xi_prime).cos() * (2.0 * 2.0 * eta_prime).sinh() +
        alpha_3 * (2.0 * 3.0 * xi_prime).cos() * (2.0 * 3.0 * eta_prime).sinh()
    ));
    let northing = 0.0 + 0.9996 * A * (xi_prime + (
        alpha_1 * (2.0 * 1.0 * xi_prime).sin() * (2.0 * 1.0 * eta_prime).cosh() +
        alpha_2 * (2.0 * 2.0 * xi_prime).sin() * (2.0 * 2.0 * eta_prime).cosh() +
        alpha_3 * (2.0 * 3.0 * xi_prime).sin() * (2.0 * 3.0 * eta_prime).cosh()
    ));

    return (northing, easting, zone);
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

    let (top_northing, left_easting, zone) = lat_long_to_utm_nad27(header.n_lat, header.w_long);
    let (bottom_northing, right_easting, _) = lat_long_to_utm_nad27(header.s_lat, header.e_long);

    let x_scale = (right_easting - left_easting) / collage_img.width() as f64;
    let y_scale = -(top_northing - bottom_northing) / collage_img.height() as f64;

    let dataset = gdal::Dataset::open_ex(
        Path::new(filename),
        Some(1),
        None,
        None,
        None,
    )?;
    dataset.set_spatial_ref(&gdal::spatial_ref::SpatialRef::from_epsg(26700 + zone)?)?;
    dataset.set_geo_transform(&[
        left_easting + x_scale / 2.0,
        x_scale,
        0.0,
        top_northing + y_scale / 2.0,
        0.0,
        y_scale,
    ])?;

    Ok(())
}
