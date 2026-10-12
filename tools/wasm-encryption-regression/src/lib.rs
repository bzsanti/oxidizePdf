#[path = "../../../oxidize-pdf-core/tests/support/issue_711_contract.rs"]
mod contract;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn error(message: &str);
}
fn hook() {
    std::panic::set_hook(Box::new(|info| error(&info.to_string())));
}
#[wasm_bindgen]
pub fn direct_iv() -> Vec<u8> {
    hook();
    contract::direct_iv()
}
#[wasm_bindgen]
pub fn authoring_case(algorithm: u32) -> Vec<u8> {
    hook();
    contract::authoring_case(algorithm)
}
