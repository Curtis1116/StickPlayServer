use image::GenericImageView;

pub fn find_crop_by_continuous_skin(
    img: &image::DynamicImage,
    width: u32,
    height: u32,
    target_width: u32,
) -> u32 {
    let max_start_x = width.saturating_sub(target_width);
    if max_start_x == 0 {
        return 0;
    }

    let model_bytes = include_bytes!("../../assets/seeta_fd_frontal_v1.0.bin");
    let mut reader = std::io::Cursor::new(&model_bytes[..]);

    if let Ok(model) = rustface::model::read_model(&mut reader) {
        let mut detector = rustface::create_detector_with_model(model);
        detector.set_min_face_size(20);
        detector.set_score_thresh(2.0);
        detector.set_pyramid_scale_factor(0.8);
        detector.set_slide_window_step(4, 4);

        let gray_img = img.to_luma8();
        let mut image_data = rustface::ImageData::new(gray_img.as_raw(), width, height);
        let faces = detector.detect(&mut image_data);

        if !faces.is_empty() {
            println!("Faces found: {}", faces.len());
            for (i, face) in faces.iter().enumerate() {
                let bbox = face.bbox();
                println!(
                    "Face {}: x={}, y={}, w={}, h={}, score={}",
                    i,
                    bbox.x(),
                    bbox.y(),
                    bbox.width(),
                    bbox.height(),
                    face.score()
                );
            }

            let largest_face = faces.iter().max_by_key(|f| {
                let bbox = f.bbox();
                bbox.width() as i64 * bbox.height() as i64
            });

            if let Some(face) = largest_face {
                let bbox = face.bbox();
                let f_x = bbox.x().max(0) as u32;
                let f_y = bbox.y().max(0) as u32;
                let f_w = bbox.width().max(0) as u32;
                let f_h = bbox.height().max(0) as u32;
                println!("Largest face: x={}, y={}, w={}, h={}", f_x, f_y, f_w, f_h);

                let min_bound = if f_x + f_w > target_width {
                    (f_x + f_w) - target_width
                } else {
                    0
                };
                let max_bound = f_x.min(max_start_x);
                println!(
                    "Constraints: min_bound={}, max_bound={}",
                    min_bound, max_bound
                );

                if min_bound <= max_bound {
                    let mut col_energy = vec![0u64; width as usize];
                    let mut col_straight_edges = vec![0u64; width as usize];

                    for y in 1..(height - 1) {
                        for x in 1..(width - 1) {
                            let p_l = img.get_pixel(x - 1, y);
                            let p_r = img.get_pixel(x + 1, y);
                            let p_u = img.get_pixel(x, y - 1);
                            let p_d = img.get_pixel(x, y + 1);

                            let lum = |p: image::Rgba<u8>| {
                                (p[0] as i32 * 299 + p[1] as i32 * 587 + p[2] as i32 * 114) / 1000
                            };

                            let l = lum(p_l);
                            let r = lum(p_r);
                            let u = lum(p_u);
                            let d = lum(p_d);

                            let gx = (r - l).abs();
                            let gy = (d - u).abs();

                            let total_gradient = gx + gy;
                            let is_straight_edge = (gx > 30 && gy < 10) || (gy > 30 && gx < 10);

                            col_energy[x as usize] += total_gradient as u64;
                            if is_straight_edge {
                                col_straight_edges[x as usize] += 1;
                            }
                        }
                    }

                    let step_x = std::cmp::max(1, width / 40);
                    let mut best_start_x = max_bound;
                    let mut min_grid_ratio = f64::MAX;

                    for start_x in (min_bound..=max_bound).step_by(step_x as usize) {
                        let mut window_energy = 0u64;
                        let mut window_straight_edges = 0u64;

                        let end_x = (start_x + target_width).min(width);
                        for x in start_x..end_x {
                            window_energy += col_energy[x as usize];
                            window_straight_edges += col_straight_edges[x as usize];
                        }

                        let energy = std::cmp::max(1, window_energy) as f64;
                        let ratio = (window_straight_edges as f64) / energy;
                        println!("start_x={}, ratio={}", start_x, ratio);

                        if ratio < min_grid_ratio {
                            min_grid_ratio = ratio;
                            best_start_x = start_x;
                        }
                    }

                    println!("Best start_x based on grid ratio: {}", best_start_x);

                    // WHAT IF we center the crop on the face?
                    let face_center_x = f_x + f_w / 2;
                    let center_start_x = face_center_x
                        .saturating_sub(target_width / 2)
                        .clamp(min_bound, max_bound);
                    println!("Centered start_x: {}", center_start_x);

                    return best_start_x;
                }
            }
        }
    }
    0
}

fn main() {
    let img_path =
        r"\\192.168.1.86\This is A Book\Classic Book\[preview]\300MIUM-1311 (明日香_S)\fanart.jpg";
    let img = image::open(img_path).expect("Failed to open image");
    let (w, h) = img.dimensions();
    println!("Image {}x{}", w, h);

    // assuming target aspect ratio 2:3 => 533x800 if height is 800
    // let's calculate target width
    let target_width = (h as f64 * 2.0 / 3.0) as u32;
    println!("Target width: {}", target_width);

    let max_start_x = w.saturating_sub(target_width);

    // Evaluate grid ratio over the entire width without face constraint
    let mut min_grid_ratio = f64::MAX;
    let mut best_start_x = max_start_x;
    let mut out_buf = String::new();

    let mut col_energy = vec![0u64; w as usize];
    let mut col_straight_edges = vec![0u64; w as usize];

    for y in 1..(h - 1) {
        for x in 1..(w - 1) {
            let p_l = img.get_pixel(x - 1, y);
            let p_r = img.get_pixel(x + 1, y);
            let p_u = img.get_pixel(x, y - 1);
            let p_d = img.get_pixel(x, y + 1);

            let lum = |p: image::Rgba<u8>| {
                (p[0] as i32 * 299 + p[1] as i32 * 587 + p[2] as i32 * 114) / 1000
            };

            let l = lum(p_l);
            let r = lum(p_r);
            let u = lum(p_u);
            let d = lum(p_d);

            let gx = (r - l).abs();
            let gy = (d - u).abs();

            let total_gradient = gx + gy;
            let is_straight_edge = (gx > 30 && gy < 10) || (gy > 30 && gx < 10);

            col_energy[x as usize] += total_gradient as u64;
            if is_straight_edge {
                col_straight_edges[x as usize] += 1;
            }
        }
    }

    let step_x = std::cmp::max(1, w / 40);
    for start_x in (0..=max_start_x).step_by(step_x as usize) {
        let mut window_energy = 0u64;
        let mut window_straight_edges = 0u64;

        let end_x = (start_x + target_width).min(w);
        for x in start_x..end_x {
            window_energy += col_energy[x as usize];
            window_straight_edges += col_straight_edges[x as usize];
        }

        let energy = std::cmp::max(1, window_energy) as f64;
        let ratio = (window_straight_edges as f64) / energy;
        out_buf.push_str(&format!("Global start_x={}, ratio={}\n", start_x, ratio));

        if ratio < min_grid_ratio {
            min_grid_ratio = ratio;
            best_start_x = start_x;
        }
    }
    out_buf.push_str(&format!(
        "Best start_x globally based on grid ratio: {}\n",
        best_start_x
    ));
    std::fs::write("global_log.txt", out_buf).unwrap();
}
