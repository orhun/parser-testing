use serde::Deserialize;

use serde_archlinux::from_str;

#[derive(Debug, Deserialize)]
struct Rofl {
    mykey: String,
    mylist: Vec<String>,
}

fn main() {
    let rofl: Rofl = from_str("any fucking str.").unwrap();
}
