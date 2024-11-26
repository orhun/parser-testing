#![allow(dead_code)]
use std::{
    fs::{self},
    io::Read,
};

use anyhow::Result;
use ariadne::{Color, Label, Report, ReportKind, Source};
use chumsky::{
    prelude::*,
    text::{ascii, newline},
    Parser,
};
use flate2::read::GzDecoder;

use winnow::{
    ascii::{alpha1, alphanumeric1, line_ending, space0, space1},
    combinator::{alt, delimited, preceded, repeat, separated_pair, terminated},
    token::take_till,
    PResult, Parser as WinnowParser,
};

/// Each line represents a line in a .MTREE file
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
enum DefaultProperty<'a> {
    Uid(usize),
    Gid(usize),
    Mode(&'a str),
    Type(PathType),
}

/// This type is used in a [Path] line and defines some available properties for that path.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Property<'a> {
    Mode(&'a str),
    Type(PathType),
    Size(usize),
    Link(&'a str),
    Sha256Digest(&'a str),
    Time(usize),
}

// What kind of type is a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathType {
    Dir,
    File,
    Link,
}

fn parser_chumsky<'a>() -> impl Parser<'a, &'a str, Vec<Statement<'a>>, extra::Err<Rich<'a, char>>>
{
    use Statement::*;

    // Parser for the very first line of the `.MTREE` file
    let mtree = just("#")
        .then(ascii::keyword("mtree"))
        .then_ignore(newline())
        .to(Init);

    // Parser for the default properties behind a `/set` or `/unset` command
    let default_properties = choice((
        // `uid` and `gid` parser that expect a user/group id.
        ascii::keyword("uid")
            .then(just('='))
            .ignore_then(text::digits(10).to_slice())
            .map(|s: &str| DefaultProperty::Uid(s.parse().unwrap())),
        ascii::keyword("gid")
            .then(just('='))
            .ignore_then(text::digits(10).to_slice())
            .map(|s: &str| DefaultProperty::Gid(s.parse().unwrap())),
        // `mode` parser which expects some octal digits
        ascii::keyword("mode")
            .then(just('='))
            .ignore_then(text::digits(8).to_slice())
            .map(|s: &str| DefaultProperty::Mode(s)),
        // `type` parser which can be one of `file`, `dir` or `link`.
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

    // The `/set` parser
    // Afterwards follows a whitespace delimited list of properties.
    let set = just("/")
        .then(ascii::keyword("set"))
        .ignore_then(default_properties)
        .map(Set);

    // The `/unset` parser
    // Afterwards follows a whitespace delimited list of properties.
    let unset = just("/")
        .ignored()
        .then_ignore(ascii::keyword("unset"))
        .to(Unset(Vec::new()));

    // Parser for the properties behind a path line
    let properties = choice((
        // `mode` parser which expects some octal digits
        ascii::keyword("mode")
            .then(just('='))
            .ignore_then(text::digits(8).to_slice())
            .map(|s: &str| Property::Mode(s)),
        // `Sha256Digest` parser which expects a hex digest
        ascii::keyword("sha256digest")
            .then(just('='))
            .ignore_then(text::digits(16).to_slice())
            .map(|s: &str| Property::Sha256Digest(s)),
        // `size` parser which expects a decimal filesize in bytes
        ascii::keyword("size")
            .then(just('='))
            .ignore_then(text::digits(10).to_slice())
            .map(|s: &str| Property::Size(s.parse().unwrap())),
        // `time` parser which expects a decimal epoch.
        // For some reason, this is a floating point number.
        // We just ignore any decimal places
        ascii::keyword("time")
            .then(just('='))
            .ignore_then(text::digits(10).to_slice())
            .then_ignore(just('.'))
            .then_ignore(text::digits(10))
            .map(|s: &str| Property::Time(s.parse().unwrap())),
        // `type` parser which can be one of `file`, `dir` or `link`.
        ascii::keyword("type")
            .then(just('='))
            .ignore_then(choice((
                ascii::keyword("dir").to(PathType::Dir),
                ascii::keyword("file").to(PathType::File),
                ascii::keyword("link").to(PathType::Link),
            )))
            .map(Property::Type),
        // `link` parser, which defines what a link links to.
        ascii::keyword("link")
            .then(just('='))
            .ignore_then(none_of(" ").repeated().to_slice())
            .map(Property::Link),
    ))
    .padded()
    .repeated()
    .collect::<Vec<_>>();

    // Parse a path line.
    // It starts with a `.` followed by some text, delimited by a whitespace.
    // TODO: Theoretically whitespaces could be inside the path?
    // Afterwards follows a whitespace delimited list of properties.
    let path = just(".")
        .then(none_of(" ").repeated().to_slice())
        .to_slice()
        .then(properties)
        .map(|(path, properties)| Path { path, properties });

    recursive(|_| choice((mtree, set, unset, path)).repeated().collect())
}

/// Parses a single value until a space or newline is encountered.
fn value<'s>(i: &mut &'s str) -> PResult<&'s str> {
    terminated(
        take_till(0.., |c| c == ' ' || c == '\n'),
        alt((space1, line_ending)),
    )
    .parse_next(i)
}

/// Parses a single value until a space or newline is encountered
///
/// This parser also consumes any trailing whitespace or newlines.
fn value_terminated<'s>(i: &mut &'s str) -> PResult<&'s str> {
    (value, alt((space0, line_ending)))
        .parse_next(i)
        .map(|(v, _)| v)
}

/// Parses the initial `#mtree` line at the top of the file.
fn init<'s>(i: &mut &'s str) -> PResult<Statement<'s>> {
    terminated("#mtree", line_ending)
        .parse_next(i)
        .map(|_| Statement::Init)
}

/// Parses a single default property.
fn default_property<'s>(i: &mut &'s str) -> PResult<DefaultProperty<'s>> {
    separated_pair(alpha1, "=", value)
        .verify_map(|(k, v)| match k {
            "uid" => Some(DefaultProperty::Uid(v.parse().ok()?)),
            "gid" => Some(DefaultProperty::Gid(v.parse().ok()?)),
            "type" => match v {
                "dir" => Some(DefaultProperty::Type(PathType::Dir)),
                "file" => Some(DefaultProperty::Type(PathType::File)),
                "link" => Some(DefaultProperty::Type(PathType::Link)),
                _ => panic!("unknown type: {v}"),
            },
            "mode" => Some(DefaultProperty::Mode(v)),
            _ => panic!("unknown property: {k}"),
        })
        .parse_next(i)
}

/// Parses a list of default properties.
fn default_properties<'s>(i: &mut &'s str) -> PResult<Vec<DefaultProperty<'s>>> {
    repeat(0.., default_property).parse_next(i)
}

/// Parses a `/set` command followed by some properties.
fn set<'s>(i: &mut &'s str) -> PResult<Statement<'s>> {
    delimited(("/set", space0), default_properties, line_ending)
        .parse_next(i)
        .map(Statement::Set)
}

/// Parses a `/unset` command followed by some properties.
fn unset<'s>(i: &mut &'s str) -> PResult<Statement<'s>> {
    delimited(("/unset", space0), default_properties, line_ending)
        .parse_next(i)
        .map(Statement::Unset)
}

/// Parses a single property.
fn property<'s>(i: &mut &'s str) -> PResult<Property<'s>> {
    separated_pair(alphanumeric1, "=", value)
        .verify_map(|(k, v)| match k {
            "type" => match v {
                "dir" => Some(Property::Type(PathType::Dir)),
                "file" => Some(Property::Type(PathType::File)),
                "link" => Some(Property::Type(PathType::Link)),
                _ => panic!("unknown type: {v}"),
            },
            "mode" => Some(Property::Mode(v)),
            "size" => Some(Property::Size(v.parse().ok()?)),
            "link" => Some(Property::Link(v)),
            "sha256digest" => Some(Property::Sha256Digest(v)),
            "time" => Some(Property::Time(v.split_once(".")?.0.parse().ok()?)),
            _ => panic!("unknown property: {k}"),
        })
        .parse_next(i)
}

/// Parses a list of properties.
fn properties<'s>(i: &mut &'s str) -> PResult<Vec<Property<'s>>> {
    repeat(0.., property).parse_next(i)
}

/// Parses a path line followed by some properties.
fn path<'s>(i: &mut &'s str) -> PResult<Statement<'s>> {
    let path = preceded(".", value).parse_next(i)?;
    let properties = delimited(' ', properties, line_ending).parse_next(i)?;
    Ok(Statement::Path { path, properties })
}

/// Parses the next statement in the file.
fn statement<'s>(i: &mut &'s str) -> PResult<Statement<'s>> {
    let statement_type: &str = alt((
        (".", value).take(),
        "/set ",
        "/unset ",
        terminated("#mtree", line_ending),
    ))
    .parse_next(i)?;

    let statement = match statement_type {
        "/set " => Statement::Set(default_properties(i)?),
        "/unset " => Statement::Unset(default_properties(i)?),
        "#mtree" => Statement::Init,
        path => Statement::Path {
            path,
            properties: properties(i)?,
        },
    };

    Ok(statement)
}

/// Parses the entire .MTREE file.
fn parser<'s>(i: &mut &'s str) -> PResult<Vec<Statement<'s>>> {
    repeat(0.., statement).parse_next(i)
}

fn main() -> Result<()> {
    let compressed = false;
    // Either read the compressed or already uncompressed .MTREE file at the root of this
    // repo and return the contents.
    let content = if compressed {
        let gz_content = fs::read(".MTREE")?;
        let mut decoder = GzDecoder::new(gz_content.as_slice());

        let mut content = String::new();
        decoder.read_to_string(&mut content).unwrap();
        content
    } else {
        String::from_utf8_lossy(&fs::read(".MTREE.extracted")?).to_string()
    };

    match parser.parse_next(&mut content.as_str()) {
        Ok(ast) => {
            println!("{:#?}", ast);
        }
        Err(e) => {
            eprintln!("{e}");
        }
    }

    // Parse the file
    let (_ast, errs) = parser_chumsky().parse(content.trim()).into_output_errors();

    // Print out the AST
    // println!("{:#?}", ast);

    // Print out any errors.
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
