use std::io::Write as _;

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn icon_bitmap(size: u32) -> Vec<u8> {
    let mut bitmap = Vec::new();
    let color_bytes = size * size * 4;
    let mask_stride = size.div_ceil(32) * 4;

    push_u32(&mut bitmap, 40);
    push_u32(&mut bitmap, size);
    push_u32(&mut bitmap, size * 2);
    push_u16(&mut bitmap, 1);
    push_u16(&mut bitmap, 32);
    push_u32(&mut bitmap, 0);
    push_u32(&mut bitmap, color_bytes);
    push_u32(&mut bitmap, 0);
    push_u32(&mut bitmap, 0);
    push_u32(&mut bitmap, 0);
    push_u32(&mut bitmap, 0);

    for y in (0..size).rev() {
        for x in 0..size {
            let mut blue = 0_u32;
            let mut green = 0_u32;
            let mut red = 0_u32;
            let mut alpha = 0_u32;
            for sample_y in 0..4 {
                for sample_x in 0..4 {
                    let px = (x as f32 + (sample_x as f32 + 0.5) / 4.0) / size as f32;
                    let py = (y as f32 + (sample_y as f32 + 0.5) / 4.0) / size as f32;
                    let dx = px - 0.5;
                    let dy = py - 0.5;
                    if dx * dx + dy * dy <= 0.46 * 0.46 {
                        let bars = [
                            (0.22, 0.31, 0.52),
                            (0.36, 0.45, 0.34),
                            (0.50, 0.59, 0.43),
                            (0.64, 0.73, 0.24),
                        ];
                        let is_bar = bars.iter().any(|&(left, right, top)| {
                            px >= left && px <= right && py >= top && py <= 0.76
                        });
                        let color = if is_bar {
                            [255, 255, 255]
                        } else {
                            [55, 125, 245]
                        };
                        red += color[0];
                        green += color[1];
                        blue += color[2];
                        alpha += 255;
                    }
                }
            }
            bitmap.extend_from_slice(&[
                (blue / 16) as u8,
                (green / 16) as u8,
                (red / 16) as u8,
                (alpha / 16) as u8,
            ]);
        }
    }
    bitmap.resize(bitmap.len() + (mask_stride * size) as usize, 0);
    bitmap
}

fn write_windows_icon(path: &std::path::Path) -> std::io::Result<()> {
    let sizes = [16_u32, 24, 32, 48, 64, 128, 256];
    let bitmaps: Vec<_> = sizes.into_iter().map(icon_bitmap).collect();
    let header_size = 6 + bitmaps.len() * 16;
    let mut bytes = Vec::new();
    push_u16(&mut bytes, 0);
    push_u16(&mut bytes, 1);
    push_u16(&mut bytes, bitmaps.len() as u16);
    let mut offset = header_size as u32;
    for (size, bitmap) in sizes.into_iter().zip(&bitmaps) {
        bytes.push(if size == 256 { 0 } else { size as u8 });
        bytes.push(if size == 256 { 0 } else { size as u8 });
        bytes.extend_from_slice(&[0, 0]);
        push_u16(&mut bytes, 1);
        push_u16(&mut bytes, 32);
        push_u32(&mut bytes, bitmap.len() as u32);
        push_u32(&mut bytes, offset);
        offset += bitmap.len() as u32;
    }
    for bitmap in bitmaps {
        bytes.extend_from_slice(&bitmap);
    }
    let mut file = std::fs::File::create(path)?;
    file.write_all(&bytes)
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let icon = output.join("resource-monitor.ico");
    write_windows_icon(&icon).expect("failed to generate Windows application icon");

    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon(icon.to_str().unwrap())
        .set("FileDescription", "Resource Monitor")
        .set("ProductName", "Resource Monitor")
        .set("CompanyName", "Mobil0010")
        .set("OriginalFilename", "ResourceMonitor.exe")
        .set("FileVersion", &version)
        .set("ProductVersion", &version);
    resource
        .compile()
        .expect("failed to embed Windows resources");
}
