#![allow(dead_code)]
use std::{
    fs::{self},
    io::Read,
};

use anyhow::Result;
use flate2::read::GzDecoder;

fn main() -> Result<()> {
    let compressed = false;
    let content = if compressed {
        let gz_content = fs::read(".MTREE")?;
        let mut decoder = GzDecoder::new(gz_content.as_slice());

        let mut content = String::new();
        decoder.read_to_string(&mut content).unwrap();
        content
    } else {
        String::from_utf8_lossy(&fs::read(".MTREE.extracted")?).to_string()
    };

    Ok(())
}
