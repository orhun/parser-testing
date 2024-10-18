#![allow(dead_code)]
use std::{
    fs::{self},
    io::Read,
};

use anyhow::Result;
use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::{prelude::*, text::ascii, Parser};
use flate2::read::GzDecoder;
use text::newline;

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

    let (ast, errs) = parser().parse(content.trim()).into_output_errors();
    println!("{:#?}", ast);
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

    let mtree = just("#")
        .then(ascii::keyword("mtree"))
        .then_ignore(newline())
        .to(Init);

    let default_properties = choice((
        ascii::keyword("uid")
            .then(just('='))
            .ignore_then(text::digits(10).to_slice())
            .map(|s: &str| DefaultProperty::Uid(s.parse().unwrap())),
        ascii::keyword("gid")
            .then(just('='))
            .ignore_then(text::digits(10).to_slice())
            .map(|s: &str| DefaultProperty::Gid(s.parse().unwrap())),
        ascii::keyword("mode")
            .then(just('='))
            .ignore_then(text::digits(8).to_slice())
            .map(|s: &str| DefaultProperty::Mode(s)),
        ascii::keyword("type")
            .then(just('='))
            .ignore_then(choice((
                ascii::keyword("dir").to(PathType::Dir),
                ascii::keyword("file").to(PathType::File),
                ascii::keyword("link").to(PathType::Link),
            )))
            .map(DefaultProperty::Type),
    ))
    .padded()
    .repeated()
    .collect::<Vec<_>>();

    let set = just("/")
        .then(ascii::keyword("set"))
        .ignore_then(default_properties)
        .map(Set);

    let unset = just("/")
        .ignored()
        .then_ignore(ascii::keyword("unset"))
        .to(Unset(Vec::new()));

    let properties = choice((
        ascii::keyword("mode")
            .then(just('='))
            .ignore_then(text::digits(8).to_slice())
            .map(|s: &str| Property::Mode(s)),
        ascii::keyword("sha256digest")
            .then(just('='))
            .ignore_then(text::digits(16).to_slice())
            .map(|s: &str| Property::Sha256Digest(s)),
        ascii::keyword("size")
            .then(just('='))
            .ignore_then(text::digits(10).to_slice())
            .map(|s: &str| Property::Size(s.parse().unwrap())),
        ascii::keyword("time")
            .then(just('='))
            .ignore_then(text::digits(10).to_slice())
            .then_ignore(just('.'))
            .then_ignore(just('0'))
            .map(|s: &str| Property::Time(s.parse().unwrap())),
        ascii::keyword("type")
            .then(just('='))
            .ignore_then(choice((
                ascii::keyword("dir").to(PathType::Dir),
                ascii::keyword("file").to(PathType::File),
                ascii::keyword("link").to(PathType::Link),
            )))
            .map(Property::Type),
        ascii::keyword("link")
            .then(just('='))
            .ignore_then(none_of(" ").repeated().to_slice())
            .map(Property::Link),
    ))
    .padded()
    .repeated()
    .collect::<Vec<_>>();

    let path = just(".")
        .then(none_of(" ").repeated().to_slice())
        .to_slice()
        .then(properties)
        .map(|(path, properties)| Path { path, properties });

    recursive(|raw| choice((mtree, set, unset, path)).repeated().collect())
}

#[derive(Debug, Clone)]
enum Statement<'a> {
    // The initial `#mtree` line at the top of the file
    Init,
    Set(Vec<DefaultProperty<'a>>),
    Unset(Vec<DefaultProperty<'a>>),
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

/// This type is used in `/set` and `/unset` commands to modify the currently active defaults.
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
