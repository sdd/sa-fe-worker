#![cfg(target_arch = "wasm32")]

use photon_rs::native::{open_image};
use wasm_bindgen_test::*;
use web_sys::ImageData;
use sa_fe_worker::process_image;

wasm_bindgen_test_configure!(run_in_node);

#[wasm_bindgen_test]
fn test_process_image() {
    let img_data: ImageData = open_image("./SAM_4298.JPG").expect("File should open").get_image_data();

    let result = process_image(img_data);

    println!("result: {:?}", &result);
}

#[wasm_bindgen_test]
fn pass() {
    assert_eq!(1, 1);
}

#[wasm_bindgen_test]
fn fail() {
    assert_eq!(1, 2);
}