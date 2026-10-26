mod control;
mod candidate;
use std::{hint::black_box, time::Instant};
fn main() {
    let text = format!("{}é", "A".repeat(16_384));
    let expected: std::collections::HashSet<char> = text.chars().collect();
    assert_eq!(control::UsedCharacters::from_iter(text.chars()).as_set(), &expected);
    assert_eq!(candidate::UsedCharacters::from_text(&text).as_set(), &expected);
    for block in 0..20 {
        let order = if block % 2 == 0 { [false, true] } else { [true, false] };
        for new in order {
            let start = Instant::now();
            for _ in 0..1000 {
                if new {
                    black_box(candidate::UsedCharacters::from_text(black_box(&text)));
                } else {
                    black_box(control::UsedCharacters::from_iter(black_box(&text).chars()));
                }
            }
            println!("{block},{new},{}", start.elapsed().as_nanos());
        }
    }
}
