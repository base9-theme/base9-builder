use std::{cell::RefCell, rc::Rc};

use crate::{config, base9};
use crate::base9::ColorMap;
use wasm_bindgen::prelude::*;

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
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet() {
    alert("Hello, wasm-test!");
}

fn palette_to_color_map(palette: &str) -> Result<Rc<RefCell<ColorMap>>, JsError> {
    let mut config = config::default_config();
    config.palette = config::Palette::from_str(palette).map_err(|x| JsError::new(&x))?;

    base9::get_variables(&config).map_err(|x| JsError::new(&x.to_string()))
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
