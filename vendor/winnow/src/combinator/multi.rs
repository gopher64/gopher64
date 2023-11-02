//! Combinators applying their child parser multiple times

use crate::error::ErrMode;
use crate::error::ErrorKind;
use crate::error::ParserError;
use crate::stream::Accumulate;
use crate::stream::Range;
use crate::stream::Stream;
use crate::trace::trace;
use crate::PResult;
use crate::Parser;

/// [`Accumulate`] the output of a parser into a container, like `Vec`
///
/// This stops before `n` when the parser returns [`ErrMode::Backtrack`].  To instead chain an error up, see
/// [`cut_err`][crate::combinator::cut_err].
///
/// # Arguments
/// * `m` The minimum number of iterations.
/// * `n` The maximum number of iterations.
/// * `f` The parser to apply.
///
/// To recognize a series of tokens, [`Accumulate`] into a `()` and then [`Parser::recognize`].
///
/// **Warning:** If the parser passed to `repeat` accepts empty inputs
/// (like `alpha0` or `digit0`), `repeat` will return an error,
/// to prevent going into an infinite loop.
///
/// # Example
///
/// Zero or more reptitions:
/// ```rust
/// # #[cfg(feature = "std")] {
/// # use winnow::{error::ErrMode, error::ErrorKind, error::Needed};
/// # use winnow::prelude::*;
/// use winnow::combinator::repeat;
/// use winnow::token::tag;
///
/// fn parser(s: &str) -> IResult<&str, Vec<&str>> {
///   repeat(0.., "abc").parse_peek(s)
/// }
///
/// assert_eq!(parser("abcabc"), Ok(("", vec!["abc", "abc"])));
/// assert_eq!(parser("abc123"), Ok(("123", vec!["abc"])));
/// assert_eq!(parser("123123"), Ok(("123123", vec![])));
/// assert_eq!(parser(""), Ok(("", vec![])));
/// # }
/// ```
///
/// One or more reptitions:
/// ```rust
/// # #[cfg(feature = "std")] {
/// # use winnow::{error::ErrMode, error::{InputError, ErrorKind}, error::Needed};
/// # use winnow::prelude::*;
/// use winnow::combinator::repeat;
/// use winnow::token::tag;
///
/// fn parser(s: &str) -> IResult<&str, Vec<&str>> {
///   repeat(1.., "abc").parse_peek(s)
/// }
///
/// assert_eq!(parser("abcabc"), Ok(("", vec!["abc", "abc"])));
/// assert_eq!(parser("abc123"), Ok(("123", vec!["abc"])));
/// assert_eq!(parser("123123"), Err(ErrMode::Backtrack(InputError::new("123123", ErrorKind::Tag))));
/// assert_eq!(parser(""), Err(ErrMode::Backtrack(InputError::new("", ErrorKind::Tag))));
/// # }
/// ```
///
/// Fixed number of repeitions:
/// ```rust
/// # #[cfg(feature = "std")] {
/// # use winnow::{error::ErrMode, error::{InputError, ErrorKind}, error::Needed};
/// # use winnow::prelude::*;
/// use winnow::combinator::repeat;
/// use winnow::token::tag;
///
/// fn parser(s: &str) -> IResult<&str, Vec<&str>> {
///   repeat(2, "abc").parse_peek(s)
/// }
///
/// assert_eq!(parser("abcabc"), Ok(("", vec!["abc", "abc"])));
/// assert_eq!(parser("abc123"), Err(ErrMode::Backtrack(InputError::new("123", ErrorKind::Tag))));
/// assert_eq!(parser("123123"), Err(ErrMode::Backtrack(InputError::new("123123", ErrorKind::Tag))));
/// assert_eq!(parser(""), Err(ErrMode::Backtrack(InputError::new("", ErrorKind::Tag))));
/// assert_eq!(parser("abcabcabc"), Ok(("abc", vec!["abc", "abc"])));
/// # }
/// ```
///
/// Arbitrary reptitions:
/// ```rust
/// # #[cfg(feature = "std")] {
/// # use winnow::{error::ErrMode, error::ErrorKind, error::Needed};
/// # use winnow::prelude::*;
/// use winnow::combinator::repeat;
/// use winnow::token::tag;
///
/// fn parser(s: &str) -> IResult<&str, Vec<&str>> {
///   repeat(0..=2, "abc").parse_peek(s)
/// }
///
/// assert_eq!(parser("abcabc"), Ok(("", vec!["abc", "abc"])));
/// assert_eq!(parser("abc123"), Ok(("123", vec!["abc"])));
/// assert_eq!(parser("123123"), Ok(("123123", vec![])));
/// assert_eq!(parser(""), Ok(("", vec![])));
/// assert_eq!(parser("abcabcabc"), Ok(("abc", vec!["abc", "abc"])));
/// # }
/// ```
#[doc(alias = "many0")]
#[doc(alias = "count")]
#[doc(alias = "many0_count")]
#[doc(alias = "many1")]
#[doc(alias = "many1_count")]
#[doc(alias = "many_m_n")]
#[doc(alias = "repeated")]
#[doc(alias = "skip_many")]
#[doc(alias = "skip_many1")]
#[inline(always)]
pub fn repeat<I, O, C, E, F>(range: impl Into<Range>, mut f: F) -> impl Parser<I, C, E>
where
    I: Stream,
    C: Accumulate<O>,
    F: Parser<I, O, E>,
    E: ParserError<I>,
{
    let Range {
        start_inclusive,
        end_inclusive,
    } = range.into();
    trace("repeat", move |i: &mut I| {
        match (start_inclusive, end_inclusive) {
            (0, None) => repeat0_(&mut f, i),
            (1, None) => repeat1_(&mut f, i),
            (start, end) if Some(start) == end => repeat_n_(start, &mut f, i),
            (start, end) => repeat_m_n_(start, end.unwrap_or(usize::MAX), &mut f, i),
        }
    })
}

fn repeat0_<I, O, C, E, F>(f: &mut F, i: &mut I) -> PResult<C, E>
where
    I: Stream,
    C: Accumulate<O>,
    F: Parser<I, O, E>,
    E: ParserError<I>,
{
    let mut acc = C::initial(None);
    loop {
        let start = i.checkpoint();
        let len = i.eof_offset();
        match f.parse_next(i) {
            Err(ErrMode::Backtrack(_)) => {
                i.reset(start);
                return Ok(acc);
            }
            Err(e) => return Err(e),
            Ok(o) => {
                // infinite loop check: the parser must always consume
                if i.eof_offset() == len {
                    return Err(ErrMode::assert(i, "`repeat` parsers must always consume"));
                }

                acc.accumulate(o);
            }
        }
    }
}

fn repeat1_<I, O, C, E, F>(f: &mut F, i: &mut I) -> PResult<C, E>
where
    I: Stream,
    C: Accumulate<O>,
    F: Parser<I, O, E>,
    E: ParserError<I>,
{
    match f.parse_next(i) {
        Err(e) => Err(e.append(i, ErrorKind::Many)),
        Ok(o) => {
            let mut acc = C::initial(None);
            acc.accumulate(o);

            loop {
                let start = i.checkpoint();
                let len = i.eof_offset();
                match f.parse_next(i) {
                    Err(ErrMode::Backtrack(_)) => {
                        i.reset(start);
                        return Ok(acc);
                    }
                    Err(e) => return Err(e),
                    Ok(o) => {
                        // infinite loop check: the parser must always consume
                        if i.eof_offset() == len {
                            return Err(ErrMode::assert(i, "`repeat` parsers must always consume"));
                        }

                        acc.accumulate(o);
                    }
                }
            }
        }
    }
}

/// [`Accumulate`] the output of parser `f` into a container, like `Vec`, until the parser `g`
/// produces a result.
///
/// Returns a tuple of the results of `f` in a `Vec` and the result of `g`.
///
/// `f` keeps going so long as `g` produces [`ErrMode::Backtrack`]. To instead chain an error up, see [`cut_err`][crate::combinator::cut_err].
///
/// To recognize a series of tokens, [`Accumulate`] into a `()` and then [`Parser::recognize`].
///
/// # Example
///
/// ```rust
/// # #[cfg(feature = "std")] {
/// # use winnow::{error::ErrMode, error::{InputError, ErrorKind}, error::Needed};
/// # use winnow::prelude::*;
/// use winnow::combinator::repeat_till0;
/// use winnow::token::tag;
///
/// fn parser(s: &str) -> IResult<&str, (Vec<&str>, &str)> {
///   repeat_till0("abc", "end").parse_peek(s)
/// };
///
/// assert_eq!(parser("abcabcend"), Ok(("", (vec!["abc", "abc"], "end"))));
/// assert_eq!(parser("abc123end"), Err(ErrMode::Backtrack(InputError::new("123end", ErrorKind::Tag))));
/// assert_eq!(parser("123123end"), Err(ErrMode::Backtrack(InputError::new("123123end", ErrorKind::Tag))));
/// assert_eq!(parser(""), Err(ErrMode::Backtrack(InputError::new("", ErrorKind::Tag))));
/// assert_eq!(parser("abcendefg"), Ok(("efg", (vec!["abc"], "end"))));
/// # }
/// ```
#[doc(alias = "many_till0")]
pub fn repeat_till0<I, O, C, P, E, F, G>(mut f: F, mut g: G) -> impl Parser<I, (C, P), E>
where
    I: Stream,
    C: Accumulate<O>,
    F: Parser<I, O, E>,
    G: Parser<I, P, E>,
    E: ParserError<I>,
{
    trace("repeat_till0", move |i: &mut I| {
        let mut res = C::initial(None);
        loop {
            let start = i.checkpoint();
            let len = i.eof_offset();
            match g.parse_next(i) {
                Ok(o) => return Ok((res, o)),
                Err(ErrMode::Backtrack(_)) => {
                    i.reset(start);
                    match f.parse_next(i) {
                        Err(e) => return Err(e.append(i, ErrorKind::Many)),
                        Ok(o) => {
                            // infinite loop check: the parser must always consume
                            if i.eof_offset() == len {
                                return Err(ErrMode::assert(
                                    i,
                                    "`repeat` parsers must always consume",
                                ));
                            }

                            res.accumulate(o);
                        }
                    }
                }
                Err(e) => return Err(e),
            }
        }
    })
}

/// [`Accumulate`] the output of a parser, interleaved with `sep`
///
/// This stops when either parser returns [`ErrMode::Backtrack`].  To instead chain an error up, see
/// [`cut_err`][crate::combinator::cut_err].
///
/// # Arguments
/// * `parser` Parses the elements of the list.
/// * `sep` Parses the separator between list elements.
///
/// # Example
///
/// ```rust
/// # #[cfg(feature = "std")] {
/// # use winnow::{error::ErrMode, error::ErrorKind, error::Needed};
/// # use winnow::prelude::*;
/// use winnow::combinator::separated0;
/// use winnow::token::tag;
///
/// fn parser(s: &str) -> IResult<&str, Vec<&str>> {
///   separated0("abc", "|").parse_peek(s)
/// }
///
/// assert_eq!(parser("abc|abc|abc"), Ok(("", vec!["abc", "abc", "abc"])));
/// assert_eq!(parser("abc123abc"), Ok(("123abc", vec!["abc"])));
/// assert_eq!(parser("abc|def"), Ok(("|def", vec!["abc"])));
/// assert_eq!(parser(""), Ok(("", vec![])));
/// assert_eq!(parser("def|abc"), Ok(("def|abc", vec![])));
/// # }
/// ```
#[doc(alias = "sep_by")]
#[doc(alias = "separated_list0")]
pub fn separated0<I, O, C, O2, E, P, S>(mut parser: P, mut sep: S) -> impl Parser<I, C, E>
where
    I: Stream,
    C: Accumulate<O>,
    P: Parser<I, O, E>,
    S: Parser<I, O2, E>,
    E: ParserError<I>,
{
    trace("separated0", move |i: &mut I| {
        let mut res = C::initial(None);

        let start = i.checkpoint();
        match parser.parse_next(i) {
            Err(ErrMode::Backtrack(_)) => {
                i.reset(start);
                return Ok(res);
            }
            Err(e) => return Err(e),
            Ok(o) => {
                res.accumulate(o);
            }
        }

        loop {
            let start = i.checkpoint();
            let len = i.eof_offset();
            match sep.parse_next(i) {
                Err(ErrMode::Backtrack(_)) => {
                    i.reset(start);
                    return Ok(res);
                }
                Err(e) => return Err(e),
                Ok(_) => {
                    // infinite loop check: the parser must always consume
                    if i.eof_offset() == len {
                        return Err(ErrMode::assert(i, "sep parsers must always consume"));
                    }

                    match parser.parse_next(i) {
                        Err(ErrMode::Backtrack(_)) => {
                            i.reset(start);
                            return Ok(res);
                        }
                        Err(e) => return Err(e),
                        Ok(o) => {
                            res.accumulate(o);
                        }
                    }
                }
            }
        }
    })
}

/// [`Accumulate`] the output of a parser, interleaved with `sep`
///
/// Fails if the element parser does not produce at least one element.$
///
/// This stops when either parser returns [`ErrMode::Backtrack`].  To instead chain an error up, see
/// [`cut_err`][crate::combinator::cut_err].
///
/// # Arguments
/// * `sep` Parses the separator between list elements.
/// * `f` Parses the elements of the list.
///
/// # Example
///
/// ```rust
/// # #[cfg(feature = "std")] {
/// # use winnow::{error::ErrMode, error::{InputError, ErrorKind}, error::Needed};
/// # use winnow::prelude::*;
/// use winnow::combinator::separated1;
/// use winnow::token::tag;
///
/// fn parser(s: &str) -> IResult<&str, Vec<&str>> {
///   separated1("abc", "|").parse_peek(s)
/// }
///
/// assert_eq!(parser("abc|abc|abc"), Ok(("", vec!["abc", "abc", "abc"])));
/// assert_eq!(parser("abc123abc"), Ok(("123abc", vec!["abc"])));
/// assert_eq!(parser("abc|def"), Ok(("|def", vec!["abc"])));
/// assert_eq!(parser(""), Err(ErrMode::Backtrack(InputError::new("", ErrorKind::Tag))));
/// assert_eq!(parser("def|abc"), Err(ErrMode::Backtrack(InputError::new("def|abc", ErrorKind::Tag))));
/// # }
/// ```
#[doc(alias = "sep_by1")]
#[doc(alias = "separated_list1")]
pub fn separated1<I, O, C, O2, E, P, S>(mut parser: P, mut sep: S) -> impl Parser<I, C, E>
where
    I: Stream,
    C: Accumulate<O>,
    P: Parser<I, O, E>,
    S: Parser<I, O2, E>,
    E: ParserError<I>,
{
    trace("separated1", move |i: &mut I| {
        let mut res = C::initial(None);

        // Parse the first element
        match parser.parse_next(i) {
            Err(e) => return Err(e),
            Ok(o) => {
                res.accumulate(o);
            }
        }

        loop {
            let start = i.checkpoint();
            let len = i.eof_offset();
            match sep.parse_next(i) {
                Err(ErrMode::Backtrack(_)) => {
                    i.reset(start);
                    return Ok(res);
                }
                Err(e) => return Err(e),
                Ok(_) => {
                    // infinite loop check: the parser must always consume
                    if i.eof_offset() == len {
                        return Err(ErrMode::assert(i, "sep parsers must always consume"));
                    }

                    match parser.parse_next(i) {
                        Err(ErrMode::Backtrack(_)) => {
                            i.reset(start);
                            return Ok(res);
                        }
                        Err(e) => return Err(e),
                        Ok(o) => {
                            res.accumulate(o);
                        }
                    }
                }
            }
        }
    })
}

/// Alternates between two parsers, merging the results (left associative)
///
/// This stops when either parser returns [`ErrMode::Backtrack`].  To instead chain an error up, see
/// [`cut_err`][crate::combinator::cut_err].
///
/// # Example
///
/// ```rust
/// # use winnow::{error::ErrMode, error::{InputError, ErrorKind}, error::Needed};
/// # use winnow::prelude::*;
/// use winnow::combinator::separated_foldl1;
/// use winnow::ascii::dec_int;
///
/// fn parser(s: &str) -> IResult<&str, i32> {
///   separated_foldl1(dec_int, "-", |l, _, r| l - r).parse_peek(s)
/// }
///
/// assert_eq!(parser("9-3-5"), Ok(("", 1)));
/// assert_eq!(parser(""), Err(ErrMode::Backtrack(InputError::new("", ErrorKind::Slice))));
/// assert_eq!(parser("def|abc"), Err(ErrMode::Backtrack(InputError::new("def|abc", ErrorKind::Slice))));
/// ```
pub fn separated_foldl1<I, O, O2, E, P, S, Op>(
    mut parser: P,
    mut sep: S,
    op: Op,
) -> impl Parser<I, O, E>
where
    I: Stream,
    P: Parser<I, O, E>,
    S: Parser<I, O2, E>,
    E: ParserError<I>,
    Op: Fn(O, O2, O) -> O,
{
    trace("separated_foldl1", move |i: &mut I| {
        let mut ol = parser.parse_next(i)?;

        loop {
            let start = i.checkpoint();
            let len = i.eof_offset();
            match sep.parse_next(i) {
                Err(ErrMode::Backtrack(_)) => {
                    i.reset(start);
                    return Ok(ol);
                }
                Err(e) => return Err(e),
                Ok(s) => {
                    // infinite loop check: the parser must always consume
                    if i.eof_offset() == len {
                        return Err(ErrMode::assert(i, "`repeat` parsers must always consume"));
                    }

                    match parser.parse_next(i) {
                        Err(ErrMode::Backtrack(_)) => {
                            i.reset(start);
                            return Ok(ol);
                        }
                        Err(e) => return Err(e),
                        Ok(or) => {
                            ol = op(ol, s, or);
                        }
                    }
                }
            }
        }
    })
}

/// Alternates between two parsers, merging the results (right associative)
///
/// This stops when either parser returns [`ErrMode::Backtrack`].  To instead chain an error up, see
/// [`cut_err`][crate::combinator::cut_err].
///
/// # Example
///
/// ```
/// # use winnow::{error::ErrMode, error::{InputError, ErrorKind}, error::Needed};
/// # use winnow::prelude::*;
/// use winnow::combinator::separated_foldr1;
/// use winnow::ascii::dec_uint;
///
/// fn parser(s: &str) -> IResult<&str, u32> {
///   separated_foldr1(dec_uint, "^", |l: u32, _, r: u32| l.pow(r)).parse_peek(s)
/// }
///
/// assert_eq!(parser("2^3^2"), Ok(("", 512)));
/// assert_eq!(parser("2"), Ok(("", 2)));
/// assert_eq!(parser(""), Err(ErrMode::Backtrack(InputError::new("", ErrorKind::Slice))));
/// assert_eq!(parser("def|abc"), Err(ErrMode::Backtrack(InputError::new("def|abc", ErrorKind::Slice))));
/// ```
#[cfg(feature = "alloc")]
pub fn separated_foldr1<I, O, O2, E, P, S, Op>(
    mut parser: P,
    mut sep: S,
    op: Op,
) -> impl Parser<I, O, E>
where
    I: Stream,
    P: Parser<I, O, E>,
    S: Parser<I, O2, E>,
    E: ParserError<I>,
    Op: Fn(O, O2, O) -> O,
{
    trace("separated_foldr1", move |i: &mut I| {
        let ol = parser.parse_next(i)?;
        let all: crate::lib::std::vec::Vec<(O2, O)> =
            repeat(0.., (sep.by_ref(), parser.by_ref())).parse_next(i)?;
        if let Some((s, or)) = all
            .into_iter()
            .rev()
            .reduce(|(sr, or), (sl, ol)| (sl, op(ol, sr, or)))
        {
            let merged = op(ol, s, or);
            Ok(merged)
        } else {
            Ok(ol)
        }
    })
}

fn repeat_m_n_<I, O, C, E, F>(min: usize, max: usize, parse: &mut F, input: &mut I) -> PResult<C, E>
where
    I: Stream,
    C: Accumulate<O>,
    F: Parser<I, O, E>,
    E: ParserError<I>,
{
    if min > max {
        return Err(ErrMode::Cut(E::from_error_kind(input, ErrorKind::Many)));
    }

    let mut res = C::initial(Some(min));
    for count in 0..max {
        let start = input.checkpoint();
        let len = input.eof_offset();
        match parse.parse_next(input) {
            Ok(value) => {
                // infinite loop check: the parser must always consume
                if input.eof_offset() == len {
                    return Err(ErrMode::assert(
                        input,
                        "`repeat` parsers must always consume",
                    ));
                }

                res.accumulate(value);
            }
            Err(ErrMode::Backtrack(e)) => {
                if count < min {
                    return Err(ErrMode::Backtrack(e.append(input, ErrorKind::Many)));
                } else {
                    input.reset(start);
                    return Ok(res);
                }
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    Ok(res)
}

fn repeat_n_<I, O, C, E, F>(count: usize, f: &mut F, i: &mut I) -> PResult<C, E>
where
    I: Stream,
    C: Accumulate<O>,
    F: Parser<I, O, E>,
    E: ParserError<I>,
{
    let mut res = C::initial(Some(count));

    for _ in 0..count {
        match f.parse_next(i) {
            Ok(o) => {
                res.accumulate(o);
            }
            Err(e) => {
                return Err(e.append(i, ErrorKind::Many));
            }
        }
    }

    Ok(res)
}

/// Repeats the embedded parser, filling the given slice with results.
///
/// This parser fails if the input runs out before the given slice is full.
///
/// # Arguments
/// * `f` The parser to apply.
/// * `buf` The slice to fill
///
/// # Example
///
/// ```rust
/// # use winnow::{error::ErrMode, error::{InputError, ErrorKind}, error::Needed};
/// # use winnow::prelude::*;
/// use winnow::combinator::fill;
/// use winnow::token::tag;
///
/// fn parser(s: &str) -> IResult<&str, [&str; 2]> {
///   let mut buf = ["", ""];
///   let (rest, ()) = fill("abc", &mut buf).parse_peek(s)?;
///   Ok((rest, buf))
/// }
///
/// assert_eq!(parser("abcabc"), Ok(("", ["abc", "abc"])));
/// assert_eq!(parser("abc123"), Err(ErrMode::Backtrack(InputError::new("123", ErrorKind::Tag))));
/// assert_eq!(parser("123123"), Err(ErrMode::Backtrack(InputError::new("123123", ErrorKind::Tag))));
/// assert_eq!(parser(""), Err(ErrMode::Backtrack(InputError::new("", ErrorKind::Tag))));
/// assert_eq!(parser("abcabcabc"), Ok(("abc", ["abc", "abc"])));
/// ```
pub fn fill<'a, I, O, E, F>(mut f: F, buf: &'a mut [O]) -> impl Parser<I, (), E> + 'a
where
    I: Stream + 'a,
    F: Parser<I, O, E> + 'a,
    E: ParserError<I> + 'a,
{
    trace("fill", move |i: &mut I| {
        for elem in buf.iter_mut() {
            match f.parse_next(i) {
                Ok(o) => {
                    *elem = o;
                }
                Err(e) => {
                    return Err(e.append(i, ErrorKind::Many));
                }
            }
        }

        Ok(())
    })
}

/// Repeats the embedded parser `m..=n` times, calling `g` to gather the results
///
/// This stops before `n` when the parser returns [`ErrMode::Backtrack`].  To instead chain an error up, see
/// [`cut_err`][crate::combinator::cut_err].
///
/// # Arguments
/// * `m` The minimum number of iterations.
/// * `n` The maximum number of iterations.
/// * `f` The parser to apply.
/// * `init` A function returning the initial value.
/// * `g` The function that combines a result of `f` with
///       the current accumulator.
///
/// **Warning:** If the parser passed to `fold_repeat` accepts empty inputs
/// (like `alpha0` or `digit0`), `fold_repeat` will return an error,
/// to prevent going into an infinite loop.
///
/// # Example
///
/// Zero or more repetitions:
/// ```rust
/// # use winnow::{error::ErrMode, error::ErrorKind, error::Needed};
/// # use winnow::prelude::*;
/// use winnow::combinator::fold_repeat;
/// use winnow::token::tag;
///
/// fn parser(s: &str) -> IResult<&str, Vec<&str>> {
///   fold_repeat(
///     0..,
///     "abc",
///     Vec::new,
///     |mut acc: Vec<_>, item| {
///       acc.push(item);
///       acc
///     }
///   ).parse_peek(s)
/// }
///
/// assert_eq!(parser("abcabc"), Ok(("", vec!["abc", "abc"])));
/// assert_eq!(parser("abc123"), Ok(("123", vec!["abc"])));
/// assert_eq!(parser("123123"), Ok(("123123", vec![])));
/// assert_eq!(parser(""), Ok(("", vec![])));
/// ```
///
/// One or more repetitions:
/// ```rust
/// # use winnow::{error::ErrMode, error::{InputError, ErrorKind}, error::Needed};
/// # use winnow::prelude::*;
/// use winnow::combinator::fold_repeat;
/// use winnow::token::tag;
///
/// fn parser(s: &str) -> IResult<&str, Vec<&str>> {
///   fold_repeat(
///     1..,
///     "abc",
///     Vec::new,
///     |mut acc: Vec<_>, item| {
///       acc.push(item);
///       acc
///     }
///   ).parse_peek(s)
/// }
///
/// assert_eq!(parser("abcabc"), Ok(("", vec!["abc", "abc"])));
/// assert_eq!(parser("abc123"), Ok(("123", vec!["abc"])));
/// assert_eq!(parser("123123"), Err(ErrMode::Backtrack(InputError::new("123123", ErrorKind::Many))));
/// assert_eq!(parser(""), Err(ErrMode::Backtrack(InputError::new("", ErrorKind::Many))));
/// ```
///
/// Arbitrary number of repetitions:
/// ```rust
/// # use winnow::{error::ErrMode, error::ErrorKind, error::Needed};
/// # use winnow::prelude::*;
/// use winnow::combinator::fold_repeat;
/// use winnow::token::tag;
///
/// fn parser(s: &str) -> IResult<&str, Vec<&str>> {
///   fold_repeat(
///     0..=2,
///     "abc",
///     Vec::new,
///     |mut acc: Vec<_>, item| {
///       acc.push(item);
///       acc
///     }
///   ).parse_peek(s)
/// }
///
/// assert_eq!(parser("abcabc"), Ok(("", vec!["abc", "abc"])));
/// assert_eq!(parser("abc123"), Ok(("123", vec!["abc"])));
/// assert_eq!(parser("123123"), Ok(("123123", vec![])));
/// assert_eq!(parser(""), Ok(("", vec![])));
/// assert_eq!(parser("abcabcabc"), Ok(("abc", vec!["abc", "abc"])));
/// ```
#[doc(alias = "fold_many0")]
#[doc(alias = "fold_many1")]
#[doc(alias = "fold_many_m_n")]
#[inline(always)]
pub fn fold_repeat<I, O, E, F, G, H, R>(
    range: impl Into<Range>,
    mut f: F,
    mut init: H,
    mut g: G,
) -> impl Parser<I, R, E>
where
    I: Stream,
    F: Parser<I, O, E>,
    G: FnMut(R, O) -> R,
    H: FnMut() -> R,
    E: ParserError<I>,
{
    let Range {
        start_inclusive,
        end_inclusive,
    } = range.into();
    trace("fold_repeat", move |i: &mut I| {
        match (start_inclusive, end_inclusive) {
            (0, None) => fold_repeat0_(&mut f, &mut init, &mut g, i),
            (1, None) => fold_repeat1_(&mut f, &mut init, &mut g, i),
            (start, end) => fold_repeat_m_n_(
                start,
                end.unwrap_or(usize::MAX),
                &mut f,
                &mut init,
                &mut g,
                i,
            ),
        }
    })
}

fn fold_repeat0_<I, O, E, F, G, H, R>(
    f: &mut F,
    init: &mut H,
    g: &mut G,
    input: &mut I,
) -> PResult<R, E>
where
    I: Stream,
    F: Parser<I, O, E>,
    G: FnMut(R, O) -> R,
    H: FnMut() -> R,
    E: ParserError<I>,
{
    let mut res = init();

    loop {
        let start = input.checkpoint();
        let len = input.eof_offset();
        match f.parse_next(input) {
            Ok(o) => {
                // infinite loop check: the parser must always consume
                if input.eof_offset() == len {
                    return Err(ErrMode::assert(
                        input,
                        "`repeat` parsers must always consume",
                    ));
                }

                res = g(res, o);
            }
            Err(ErrMode::Backtrack(_)) => {
                input.reset(start);
                return Ok(res);
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
}

fn fold_repeat1_<I, O, E, F, G, H, R>(
    f: &mut F,
    init: &mut H,
    g: &mut G,
    input: &mut I,
) -> PResult<R, E>
where
    I: Stream,
    F: Parser<I, O, E>,
    G: FnMut(R, O) -> R,
    H: FnMut() -> R,
    E: ParserError<I>,
{
    let init = init();
    match f.parse_next(input) {
        Err(ErrMode::Backtrack(_)) => Err(ErrMode::from_error_kind(input, ErrorKind::Many)),
        Err(e) => Err(e),
        Ok(o1) => {
            let mut acc = g(init, o1);

            loop {
                let start = input.checkpoint();
                let len = input.eof_offset();
                match f.parse_next(input) {
                    Err(ErrMode::Backtrack(_)) => {
                        input.reset(start);
                        break;
                    }
                    Err(e) => return Err(e),
                    Ok(o) => {
                        // infinite loop check: the parser must always consume
                        if input.eof_offset() == len {
                            return Err(ErrMode::assert(
                                input,
                                "`repeat` parsers must always consume",
                            ));
                        }

                        acc = g(acc, o);
                    }
                }
            }

            Ok(acc)
        }
    }
}

fn fold_repeat_m_n_<I, O, E, F, G, H, R>(
    min: usize,
    max: usize,
    parse: &mut F,
    init: &mut H,
    fold: &mut G,
    input: &mut I,
) -> PResult<R, E>
where
    I: Stream,
    F: Parser<I, O, E>,
    G: FnMut(R, O) -> R,
    H: FnMut() -> R,
    E: ParserError<I>,
{
    if min > max {
        return Err(ErrMode::Cut(E::from_error_kind(input, ErrorKind::Many)));
    }

    let mut acc = init();
    for count in 0..max {
        let start = input.checkpoint();
        let len = input.eof_offset();
        match parse.parse_next(input) {
            Ok(value) => {
                // infinite loop check: the parser must always consume
                if input.eof_offset() == len {
                    return Err(ErrMode::assert(
                        input,
                        "`repeat` parsers must always consume",
                    ));
                }

                acc = fold(acc, value);
            }
            //FInputXMError: handle failure properly
            Err(ErrMode::Backtrack(err)) => {
                if count < min {
                    return Err(ErrMode::Backtrack(err.append(input, ErrorKind::Many)));
                } else {
                    input.reset(start);
                    break;
                }
            }
            Err(e) => return Err(e),
        }
    }

    Ok(acc)
}
