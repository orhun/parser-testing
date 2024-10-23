use serde::Deserialize;

use serde_archlinux::from_str;

#[derive(Debug, Deserialize, PartialEq)]
struct Rofl {
    key: String,
    list: Vec<String>,
    u32: u32,
    u64: u32,
    i64: i64,
    i32: i32,
    single_key_list: Vec<String>,
}

fn main() {
    let deserialized: Rofl = from_str("any fucking str.").unwrap();

    assert_eq!(
        Rofl {
            key: "value".to_string(),
            list: vec!["1".to_string(), "2".to_string()],
            single_key_list: vec!["yo".to_string()],
            u64: 1,
            u32: 10,
            i64: -1,
            i32: -10,
        },
        deserialized,
    );

    println!("{deserialized:?}");
}
