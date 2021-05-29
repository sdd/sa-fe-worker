mod utils;

use photon_rs::PhotonImage;
use wasm_bindgen::prelude::*;
use web_sys::{ImageData};
use serde::{Serialize, Deserialize};

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

const POINT_EXCLUSION_RADIUS: f64 = 20.0;
const POINT_THRESHOLD: u8 = 75;
pub const PATCH_SIZE: u32 = 20;

pub const MEDIAN_RADIUS: usize = 40;
pub const MEDIAN_WINDOW_SIZE: usize = MEDIAN_RADIUS * 2 + 1;

#[wasm_bindgen]
#[derive(Serialize, Deserialize)]
pub struct QueryPointCandidate {
    pub x: u32,
    pub y: u32,
    pub val: u8,
}

enum State {
    Searching,
    WithinNewPoint{ x: u32, y: u32, max_val: u8 },
    WithinExistingPoint(usize),
    WithinGuard(usize),
}

struct HistSet {
    data: [u32; 256],
    count: u32
}

impl HistSet {
    fn new() -> HistSet {
        HistSet {
            data: [0; 256],
            count: 0
        }
    }

    fn incr(&mut self, bin: u8) {
        self.data[bin as usize] += 1;
        self.count += 1;
    }

    fn decr(&mut self, bin: u8) {
        self.data[bin as usize] -= 1;
        self.count -= 1;
    }

    fn median(&self) -> u8 {
        let mut count = 0;
        for i in 0..256 {
            count += self.data[i];

            if 2 * count >= self.count {
                return i as u8;
            }
        }

        255
    }
}

#[wasm_bindgen]
pub fn process_image(img_data: ImageData) -> JsValue {
    let img: PhotonImage = img_data.into();

    let mut points: Vec<QueryPointCandidate> = vec![];
    let mut recent_points: Vec<usize> = vec![];

    let width = img.get_width() as usize;
    let _height = img.get_height() as usize;
    let pixels = img.get_raw_pixels();

    let end = pixels.len() - 4;
    println!("end: {}", end);

    let mut greyscale_value_cache: [u8; MEDIAN_WINDOW_SIZE] = [0; MEDIAN_WINDOW_SIZE];
    let mut greyscale_cache_index = 0;

    let mut state: State = State::Searching;
    let mut hist: HistSet = HistSet::new();
    let mut x = 0;
    let mut y = 0;
    for i in (0..end).step_by(4) {
        let greyscale_val: u8 = get_greyscale_value(&pixels, i);

        while !recent_points.is_empty() && points[recent_points[0]].y < y - POINT_EXCLUSION_RADIUS as u32 {
            recent_points.remove(0);
        }

        if x == 0 {
            // reset histogram
            hist = HistSet::new();
            greyscale_cache_index = 0;

            // prepopulate cache and histogram
            for j in 0..MEDIAN_WINDOW_SIZE {
                let greyscale_val: u8 = get_greyscale_value(&pixels, i + j);
                greyscale_value_cache[j] = greyscale_val;
                hist.incr(greyscale_val);
            }
        } else if x > MEDIAN_RADIUS && x < width - MEDIAN_RADIUS {
            // remove trailing histogram entry
            greyscale_cache_index += 1;
            if greyscale_cache_index >= MEDIAN_WINDOW_SIZE {
                greyscale_cache_index = 0
            };
            hist.decr(greyscale_value_cache[greyscale_cache_index]);

            // insert leading histogram entry
            greyscale_value_cache[greyscale_cache_index] = greyscale_val;
            hist.incr(greyscale_val);
        }

        let processed_val = greyscale_val.saturating_sub(hist.median());

        state = match state {

            State::Searching => {
                let matching_point: Option<usize> = recent_points.iter().find(|&p| {
                    let p_x: u32 = points[*p].x;
                    (p_x as i32 - x as i32).abs() < POINT_EXCLUSION_RADIUS as i32
                }).map(|x|*x);

                if matching_point.is_some() {
                    State::WithinExistingPoint(matching_point.unwrap())
                } else if processed_val > POINT_THRESHOLD {
                    State::WithinNewPoint { x: x as u32, y: y as u32, max_val: processed_val }
                } else {
                    // TODO: check if within guard dist of open point from prev row.
                    //       if so, transition to guard
                    State::Searching
                }
            },

            State::WithinExistingPoint(index) => {
                let QueryPointCandidate { val, .. } = points[index];
                if processed_val > val {
                    points[index] = QueryPointCandidate { x: x as u32, y: y as u32, val: processed_val };
                    State::WithinExistingPoint(index)
                } else if processed_val > POINT_THRESHOLD {
                    State::WithinExistingPoint(index)
                } else {
                    State::WithinGuard(POINT_EXCLUSION_RADIUS as usize)
                }
            },

            State::WithinNewPoint { x: pt_x, y: pt_y, max_val } => {
                if processed_val > max_val {
                    State::WithinNewPoint { x: x as u32, y: y as u32, max_val: processed_val }
                } else if processed_val > POINT_THRESHOLD {
                    State::WithinNewPoint { x: pt_x, y: pt_y, max_val }
                } else {
                    points.push(QueryPointCandidate {
                        x: pt_x,
                        y: pt_y,
                        val: max_val
                    });
                    recent_points.push(points.len() - 1);
                    State::WithinGuard(POINT_EXCLUSION_RADIUS as usize)
                }
            },

            State::WithinGuard(remaining) => {
                if remaining > 0 {
                    State::WithinGuard(remaining - 1)
                } else {
                    State::Searching
                }
            }
        };

        x = if x + 1 == width { y += 1; 0 } else { x + 1 };
    }

    // TODO: fit points?
    JsValue::from_serde(&points).unwrap()
}

fn get_greyscale_value(pixels: &Vec<u8>, i: usize) -> u8 {
    let r_val = pixels[i] as u32;
    let g_val = pixels[i + 1] as u32;
    let b_val = pixels[i + 2] as u32;
    ((r_val + g_val + b_val) / 3).min(255) as u8
}
