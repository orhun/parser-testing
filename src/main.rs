use std::{
    fs::{self},
    io::Read,
};

use anyhow::Result;
use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::{prelude::*, text::ascii, Parser};
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

    //println!("{}", content);

    let (json, errs) = parser().parse(content.trim()).into_output_errors();
    println!("{:#?}", json);
    errs.into_iter().for_each(|e| {
        Report::build(ReportKind::Error, (), e.span().start)
            .with_message(e.to_string())
            .with_label(
                Label::new(e.span().into_range())
                    .with_message(e.reason().to_string())
                    .with_color(Color::Red),
            )
            .finish()
            .print(Source::from(&content))
            .unwrap()
    });

    Ok(())
}

fn parser<'a>() -> impl Parser<'a, &'a str, Vec<Statement<'a>>, extra::Err<Rich<'a, char>>> {
    use Statement::*;

    let mtree = just("#").then(ascii::keyword("mtree")).to(Init);

    let set = just("/")
        .ignored()
        .then_ignore(ascii::keyword("set"))
        .to(Set(Vec::new()));

    let unset = just("/")
        .ignored()
        .then_ignore(ascii::keyword("unset"))
        .to(Unset(Vec::new()));

    recursive(|raw| choice((mtree, set, unset)).repeated().collect())
}

#[derive(Debug, Clone)]
enum Statement<'a> {
    // The initial `#mtree` line at the top of the file
    Init,
    Set(Vec<DefaultProperty>),
    Unset(Vec<DefaultProperty>),
    Path {
        path: &'a str,
        properties: Vec<Property<'a>>,
    },
}

/// This type is used in `/set` and `/unset` commands to modify the currently active defaults.
#[derive(Debug, Clone)]
enum DefaultProperty {
    Uid(usize),
    Gid(usize),
    Mode(usize),
}

/// This type is used in `/set` and `/unset` commands to modify the currently active defaults.
#[derive(Debug, Clone)]
enum Property<'a> {
    Mode,
    Type(PathType),
    Size(usize),
    Link(&'a str),
    Sha256Digest(&'a str),
}

// What kind of type is a path.
#[derive(Debug, Clone, Copy)]
enum PathType {
    Dir,
    File,
    Link,
}
