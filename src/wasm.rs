use std::{cell::RefCell, rc::Rc};

use crate::{config, base9};
use crate::base9::ColorMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = r#"

type NestedObj<T> = {
    [index: string]: T|NestedObj<T>;
}

type Formatted = {
    hex: string,
    hex_r: string,
    hex_g: string,
    hex_b: string,
    int_r: string,
    int_g: string,
    int_b: string,
    dec_r: string,
    dec_g: string,
    dec_b: string,
}

type ProgrammableEntry = {
    path: {
        dotted: string,
    },
    color?: Formatted,
}

type Data = {
    PALETTE: string,
    PROGRAMMABLE: ProgrammableEntry[],
    [index: string]: NestedObj<Formatted>|string|ProgrammableEntry[],
}

"#;

pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}


// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
// #[cfg(feature = "wee_alloc")]
// #[global_allocator]
// static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet() {
    alert("Hello, wasm-test!");
}

#[wasm_bindgen]
pub fn palette_to_data(palette: &str) -> Result<JsValue, JsError> {
    let mut config = config::default_config();
    config.palette = config::Palette::from_str(palette).map_err(|x| JsError::new(&x))?;

    let variables = base9::get_variables(&config).map_err(|x| JsError::new(&x.to_string()))?;
    let formatted_variables = base9::format_variables(&config, &variables);
    JsValue::from_serde(&formatted_variables).map_err(|x| JsError::new(&x.to_string()))
}

#[wasm_bindgen]
pub fn palette_to_color_hash(palette: &str) -> Result<JsValue, JsError> {
    let mut config = config::default_config();
    config.palette = config::Palette::from_str(palette).map_err(|x| JsError::new(&x))?;

    let variables = base9::get_variables(&config).map_err(|x| JsError::new(&x.to_string()))?;
    let formatted_variables = base9::map_color_map(&variables, |c| format!("#{:x}", c).into());
    JsValue::from_serde(&formatted_variables).map_err(|x| JsError::new(&x.to_string()))
}

pub fn render_str(palette: &str) -> Result<JsValue, JsError> {
    Ok("".into())
}
