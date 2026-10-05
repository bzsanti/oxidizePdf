pub use oxidize_pdf::parser;
#[path = "../../../target/f09-index-before.rs"]
mod before;
fn main() {
    let inputs = [vec![0,1,1,0,0],vec![0,2,1,1,3,2,1,2],vec![0,1,4,0,0,0,1,255,255,255,255]];
    for data in inputs {
        let old = std::panic::catch_unwind(|| before::parse_cff_index(&data,0).is_ok());
        let fixed = oxidize_pdf::text::fonts::cff::index::parse_cff_index(&data,0).is_err();
        println!("data={data:?} before={old:?} fixed_rejects={fixed}");
        assert!(old.is_err() || matches!(old, Ok(true)));
        assert!(fixed);
    }
}
