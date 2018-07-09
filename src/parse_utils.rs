//! Utility parsers which are particularly useful when building line-based
//! parsers.

use combine::Stream;
use combine::ParseError;

use combine::char::newline;
use combine::{Parser, many, many1, none_of, try, eof};

/// Match a single non-newline character
///
/// # Examples
//
/// ```
/// # extern crate combine;
/// # extern crate rherkin;
/// # use combine::*;
/// # use rherkin::parse_utils::non_newline;
/// # fn main() {
/// let mut parser = non_newline();
/// let result_ok = parser.parse("a");
/// assert_eq!(result_ok, Ok(('a', "")));
/// let result_err = parser.parse("\n");
/// assert!(result_err.is_err());
/// # }
/// ```
pub fn non_newline<I>() -> impl Parser<Input = I, Output = char>
where I: Stream<Item = char>,
      I::Error: ParseError<I::Item, I::Range, I::Position>
{
    none_of("\r\n".chars())
}

/// Parse one or more characters up to the next newline character (or until
/// eof), consuming it. Return the characters consumed on the way to the
/// newline, but not the newline.
///
/// # Examples
//
/// ```
/// # extern crate combine;
/// # extern crate rherkin;
/// # use combine::*;
/// # use rherkin::parse_utils::until_eol;
/// # fn main() {
/// let mut parser = until_eol();
/// let result = parser.parse("abc\ndef");
/// assert_eq!(result, Ok(("abc".to_string(), "def")));
/// # }
/// ```
pub fn until_eol<I>() -> impl Parser<Input = I, Output = String>
where I: Stream<Item = char>,
      I::Error: ParseError<I::Item, I::Range, I::Position>
{
    ( many1(non_newline()), choice! { newline().map(|_| ()), eof() } )
        .map(|(s, _)| s)
}


/// Parse a block of lines using `until_eol`. Return them in string form, with
/// newlines interposed where they fall in the input, but not at the end.
///
/// # Examples
//
/// ```
/// # extern crate combine;
/// # extern crate rherkin;
/// # use combine::*;
/// # use rherkin::parse_utils::line_block;
/// # fn main() {
/// let mut parser = line_block();
/// let result = parser.parse("abc\ndef\n\nghi");
/// assert_eq!(result, Ok(("abc\ndef".to_string(), "\nghi")));
/// # }
/// ```
pub fn line_block<I>() -> impl Parser<Input = I, Output = String>
where I: Stream<Item = char>,
      I::Error: ParseError<I::Item, I::Range, I::Position>
{
    many(try(until_eol()))
        .map(|lines: Vec<String>| lines.join("\n"))
}
