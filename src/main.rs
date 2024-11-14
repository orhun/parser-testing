#![allow(dead_code)]
use std::{
    fs::{self},
    io::Read,
};

use anyhow::Result;
use flate2::read::GzDecoder;

/// Each line represents a line in a .MTREE file
#[derive(Debug, Clone)]
enum Statement<'a> {
    /// The initial `#mtree` line at the top of the file
    Init,
    /// A `/set` command followed by some properties
    Set(Vec<DefaultProperty<'a>>),
    /// A `/unset` command followed by some properties
    Unset(Vec<DefaultProperty<'a>>),
    /// Any path statement followed by some properties
    Path {
        path: &'a str,
        properties: Vec<Property<'a>>,
    },
}

/// This type is used in `/set` and `/unset` commands to modify the currently active defaults.
#[derive(Debug, Clone)]
enum DefaultProperty<'a> {
    Uid(usize),
    Gid(usize),
    Mode(&'a str),
    Type(PathType),
}

/// This type is used in a [Path] line and defines some available properties for that path.
#[derive(Debug, Clone)]
enum Property<'a> {
    Mode(&'a str),
    Type(PathType),
    Size(usize),
    Link(&'a str),
    Sha256Digest(&'a str),
    Time(usize),
}

// What kind of type is a path.
#[derive(Debug, Clone, Copy)]
enum PathType {
    Dir,
    File,
    Link,
}

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
