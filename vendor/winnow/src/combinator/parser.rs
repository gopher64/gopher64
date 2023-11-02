use crate::error::{AddContext, ErrMode, ErrorKind, FromExternalError, ParserError};
use crate::lib::std::borrow::Borrow;
use crate::lib::std::ops::Range;
use crate::stream::StreamIsPartial;
use crate::stream::{Location, Stream};
use crate::trace::trace;
use crate::trace::trace_result;
use crate::*;

/// Implementation of [`Parser::by_ref`]
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct ByRef<'p, P> {
    p: &'p mut P,
}

impl<'p, P> ByRef<'p, P> {
    #[inline(always)]
    pub(crate) fn new(p: &'p mut P) -> Self {
        Self { p }
    }
}

impl<'p, I, O, E, P> Parser<I, O, E> for ByRef<'p, P>
where
    P: Parser<I, O, E>,
{
    #[inline(always)]
    fn parse_next(&mut self, i: &mut I) -> PResult<O, E> {
        self.p.parse_next(i)
    }
}

/// Implementation of [`Parser::map`]
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct Map<F, G, I, O, O2, E>
where
    F: Parser<I, O, E>,
    G: Fn(O) -> O2,
{
    parser: F,
    map: G,
    i: core::marker::PhantomData<I>,
    o: core::marker::PhantomData<O>,
    o2: core::marker::PhantomData<O2>,
    e: core::marker::PhantomData<E>,
}

impl<F, G, I, O, O2, E> Map<F, G, I, O, O2, E>
where
    F: Parser<I, O, E>,
    G: Fn(O) -> O2,
{
    #[inline(always)]
    pub(crate) fn new(parser: F, map: G) -> Self {
        Self {
            parser,
            map,
            i: Default::default(),
            o: Default::default(),
            o2: Default::default(),
            e: Default::default(),
        }
    }
}

impl<F, G, I, O, O2, E> Parser<I, O2, E> for Map<F, G, I, O, O2, E>
where
    F: Parser<I, O, E>,
    G: Fn(O) -> O2,
{
    #[inline]
    fn parse_next(&mut self, i: &mut I) -> PResult<O2, E> {
        match self.parser.parse_next(i) {
            Err(e) => Err(e),
            Ok(o) => Ok((self.map)(o)),
        }
    }
}

/// Implementation of [`Parser::try_map`]
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct TryMap<F, G, I, O, O2, E, E2>
where
    F: Parser<I, O, E>,
    G: FnMut(O) -> Result<O2, E2>,
    I: Stream,
    E: FromExternalError<I, E2>,
{
    parser: F,
    map: G,
    i: core::marker::PhantomData<I>,
    o: core::marker::PhantomData<O>,
    o2: core::marker::PhantomData<O2>,
    e: core::marker::PhantomData<E>,
    e2: core::marker::PhantomData<E2>,
}

impl<F, G, I, O, O2, E, E2> TryMap<F, G, I, O, O2, E, E2>
where
    F: Parser<I, O, E>,
    G: FnMut(O) -> Result<O2, E2>,
    I: Stream,
    E: FromExternalError<I, E2>,
{
    #[inline(always)]
    pub(crate) fn new(parser: F, map: G) -> Self {
        Self {
            parser,
            map,
            i: Default::default(),
            o: Default::default(),
            o2: Default::default(),
            e: Default::default(),
            e2: Default::default(),
        }
    }
}

impl<F, G, I, O, O2, E, E2> Parser<I, O2, E> for TryMap<F, G, I, O, O2, E, E2>
where
    F: Parser<I, O, E>,
    G: FnMut(O) -> Result<O2, E2>,
    I: Stream,
    E: FromExternalError<I, E2>,
{
    #[inline]
    fn parse_next(&mut self, input: &mut I) -> PResult<O2, E> {
        let start = input.checkpoint();
        let o = self.parser.parse_next(input)?;
        let res = (self.map)(o).map_err(|err| {
            input.reset(start);
            ErrMode::from_external_error(input, ErrorKind::Verify, err)
        });
        trace_result("verify", &res);
        res
    }
}

/// Implementation of [`Parser::verify_map`]
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct VerifyMap<F, G, I, O, O2, E>
where
    F: Parser<I, O, E>,
    G: FnMut(O) -> Option<O2>,
    I: Stream,
    E: ParserError<I>,
{
    parser: F,
    map: G,
    i: core::marker::PhantomData<I>,
    o: core::marker::PhantomData<O>,
    o2: core::marker::PhantomData<O2>,
    e: core::marker::PhantomData<E>,
}

impl<F, G, I, O, O2, E> VerifyMap<F, G, I, O, O2, E>
where
    F: Parser<I, O, E>,
    G: FnMut(O) -> Option<O2>,
    I: Stream,
    E: ParserError<I>,
{
    #[inline(always)]
    pub(crate) fn new(parser: F, map: G) -> Self {
        Self {
            parser,
            map,
            i: Default::default(),
            o: Default::default(),
            o2: Default::default(),
            e: Default::default(),
        }
    }
}

impl<F, G, I, O, O2, E> Parser<I, O2, E> for VerifyMap<F, G, I, O, O2, E>
where
    F: Parser<I, O, E>,
    G: FnMut(O) -> Option<O2>,
    I: Stream,
    E: ParserError<I>,
{
    #[inline]
    fn parse_next(&mut self, input: &mut I) -> PResult<O2, E> {
        let start = input.checkpoint();
        let o = self.parser.parse_next(input)?;
        let res = (self.map)(o).ok_or_else(|| {
            input.reset(start);
            ErrMode::from_error_kind(input, ErrorKind::Verify)
        });
        trace_result("verify", &res);
        res
    }
}

/// Implementation of [`Parser::and_then`]
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct AndThen<F, G, I, O, O2, E>
where
    F: Parser<I, O, E>,
    G: Parser<O, O2, E>,
    O: StreamIsPartial,
    I: Stream,
{
    outer: F,
    inner: G,
    i: core::marker::PhantomData<I>,
    o: core::marker::PhantomData<O>,
    o2: core::marker::PhantomData<O2>,
    e: core::marker::PhantomData<E>,
}

impl<F, G, I, O, O2, E> AndThen<F, G, I, O, O2, E>
where
    F: Parser<I, O, E>,
    G: Parser<O, O2, E>,
    O: StreamIsPartial,
    I: Stream,
{
    #[inline(always)]
    pub(crate) fn new(outer: F, inner: G) -> Self {
        Self {
            outer,
            inner,
            i: Default::default(),
            o: Default::default(),
            o2: Default::default(),
            e: Default::default(),
        }
    }
}

impl<F, G, I, O, O2, E> Parser<I, O2, E> for AndThen<F, G, I, O, O2, E>
where
    F: Parser<I, O, E>,
    G: Parser<O, O2, E>,
    O: StreamIsPartial,
    I: Stream,
{
    #[inline(always)]
    fn parse_next(&mut self, i: &mut I) -> PResult<O2, E> {
        let start = i.checkpoint();
        let mut o = self.outer.parse_next(i)?;
        let _ = o.complete();
        let o2 = self.inner.parse_next(&mut o).map_err(|err| {
            i.reset(start);
            err
        })?;
        Ok(o2)
    }
}

/// Implementation of [`Parser::parse_to`]
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct ParseTo<P, I, O, O2, E>
where
    P: Parser<I, O, E>,
    I: Stream,
    O: crate::stream::ParseSlice<O2>,
    E: ParserError<I>,
{
    p: P,
    i: core::marker::PhantomData<I>,
    o: core::marker::PhantomData<O>,
    o2: core::marker::PhantomData<O2>,
    e: core::marker::PhantomData<E>,
}

impl<P, I, O, O2, E> ParseTo<P, I, O, O2, E>
where
    P: Parser<I, O, E>,
    I: Stream,
    O: crate::stream::ParseSlice<O2>,
    E: ParserError<I>,
{
    #[inline(always)]
    pub(crate) fn new(p: P) -> Self {
        Self {
            p,
            i: Default::default(),
            o: Default::default(),
            o2: Default::default(),
            e: Default::default(),
        }
    }
}

impl<P, I, O, O2, E> Parser<I, O2, E> for ParseTo<P, I, O, O2, E>
where
    P: Parser<I, O, E>,
    I: Stream,
    O: crate::stream::ParseSlice<O2>,
    E: ParserError<I>,
{
    #[inline]
    fn parse_next(&mut self, i: &mut I) -> PResult<O2, E> {
        let start = i.checkpoint();
        let o = self.p.parse_next(i)?;
        let res = o.parse_slice().ok_or_else(|| {
            i.reset(start);
            ErrMode::from_error_kind(i, ErrorKind::Verify)
        });
        trace_result("verify", &res);
        res
    }
}

/// Implementation of [`Parser::flat_map`]
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct FlatMap<F, G, H, I, O, O2, E>
where
    F: Parser<I, O, E>,
    G: FnMut(O) -> H,
    H: Parser<I, O2, E>,
{
    f: F,
    g: G,
    h: core::marker::PhantomData<H>,
    i: core::marker::PhantomData<I>,
    o: core::marker::PhantomData<O>,
    o2: core::marker::PhantomData<O2>,
    e: core::marker::PhantomData<E>,
}

impl<F, G, H, I, O, O2, E> FlatMap<F, G, H, I, O, O2, E>
where
    F: Parser<I, O, E>,
    G: FnMut(O) -> H,
    H: Parser<I, O2, E>,
{
    #[inline(always)]
    pub(crate) fn new(f: F, g: G) -> Self {
        Self {
            f,
            g,
            h: Default::default(),
            i: Default::default(),
            o: Default::default(),
            o2: Default::default(),
            e: Default::default(),
        }
    }
}

impl<F, G, H, I, O, O2, E> Parser<I, O2, E> for FlatMap<F, G, H, I, O, O2, E>
where
    F: Parser<I, O, E>,
    G: FnMut(O) -> H,
    H: Parser<I, O2, E>,
{
    #[inline(always)]
    fn parse_next(&mut self, i: &mut I) -> PResult<O2, E> {
        let o = self.f.parse_next(i)?;
        (self.g)(o).parse_next(i)
    }
}

/// Implementation of [`Parser::complete_err`]
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct CompleteErr<F> {
    f: F,
}

impl<F> CompleteErr<F> {
    #[inline(always)]
    pub(crate) fn new(f: F) -> Self {
        Self { f }
    }
}

impl<F, I, O, E> Parser<I, O, E> for CompleteErr<F>
where
    I: Stream,
    F: Parser<I, O, E>,
    E: ParserError<I>,
{
    #[inline]
    fn parse_next(&mut self, input: &mut I) -> PResult<O, E> {
        trace("complete_err", |input: &mut I| {
            match (self.f).parse_next(input) {
                Err(ErrMode::Incomplete(_)) => {
                    Err(ErrMode::from_error_kind(input, ErrorKind::Complete))
                }
                rest => rest,
            }
        })
        .parse_next(input)
    }
}

/// Implementation of [`Parser::verify`]
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct Verify<F, G, I, O, O2, E>
where
    F: Parser<I, O, E>,
    G: Fn(&O2) -> bool,
    I: Stream,
    O: Borrow<O2>,
    O2: ?Sized,
    E: ParserError<I>,
{
    parser: F,
    filter: G,
    i: core::marker::PhantomData<I>,
    o: core::marker::PhantomData<O>,
    o2: core::marker::PhantomData<O2>,
    e: core::marker::PhantomData<E>,
}

impl<F, G, I, O, O2, E> Verify<F, G, I, O, O2, E>
where
    F: Parser<I, O, E>,
    G: Fn(&O2) -> bool,
    I: Stream,
    O: Borrow<O2>,
    O2: ?Sized,
    E: ParserError<I>,
{
    #[inline(always)]
    pub(crate) fn new(parser: F, filter: G) -> Self {
        Self {
            parser,
            filter,
            i: Default::default(),
            o: Default::default(),
            o2: Default::default(),
            e: Default::default(),
        }
    }
}

impl<F, G, I, O, O2, E> Parser<I, O, E> for Verify<F, G, I, O, O2, E>
where
    F: Parser<I, O, E>,
    G: Fn(&O2) -> bool,
    I: Stream,
    O: Borrow<O2>,
    O2: ?Sized,
    E: ParserError<I>,
{
    #[inline]
    fn parse_next(&mut self, input: &mut I) -> PResult<O, E> {
        let start = input.checkpoint();
        let o = self.parser.parse_next(input)?;
        let res = (self.filter)(o.borrow()).then_some(o).ok_or_else(|| {
            input.reset(start);
            ErrMode::from_error_kind(input, ErrorKind::Verify)
        });
        trace_result("verify", &res);
        res
    }
}

/// Implementation of [`Parser::value`]
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct Value<F, I, O, O2, E>
where
    F: Parser<I, O, E>,
    O2: Clone,
{
    parser: F,
    val: O2,
    i: core::marker::PhantomData<I>,
    o: core::marker::PhantomData<O>,
    e: core::marker::PhantomData<E>,
}

impl<F, I, O, O2, E> Value<F, I, O, O2, E>
where
    F: Parser<I, O, E>,
    O2: Clone,
{
    #[inline(always)]
    pub(crate) fn new(parser: F, val: O2) -> Self {
        Self {
            parser,
            val,
            i: Default::default(),
            o: Default::default(),
            e: Default::default(),
        }
    }
}

impl<F, I, O, O2, E> Parser<I, O2, E> for Value<F, I, O, O2, E>
where
    F: Parser<I, O, E>,
    O2: Clone,
{
    #[inline]
    fn parse_next(&mut self, input: &mut I) -> PResult<O2, E> {
        (self.parser).parse_next(input).map(|_| self.val.clone())
    }
}

/// Implementation of [`Parser::void`]
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct Void<F, I, O, E>
where
    F: Parser<I, O, E>,
{
    parser: F,
    i: core::marker::PhantomData<I>,
    o: core::marker::PhantomData<O>,
    e: core::marker::PhantomData<E>,
}

impl<F, I, O, E> Void<F, I, O, E>
where
    F: Parser<I, O, E>,
{
    #[inline(always)]
    pub(crate) fn new(parser: F) -> Self {
        Self {
            parser,
            i: Default::default(),
            o: Default::default(),
            e: Default::default(),
        }
    }
}

impl<F, I, O, E> Parser<I, (), E> for Void<F, I, O, E>
where
    F: Parser<I, O, E>,
{
    #[inline(always)]
    fn parse_next(&mut self, input: &mut I) -> PResult<(), E> {
        (self.parser).parse_next(input).map(|_| ())
    }
}

/// Implementation of [`Parser::recognize`]
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct Recognize<F, I, O, E>
where
    F: Parser<I, O, E>,
    I: Stream,
{
    parser: F,
    i: core::marker::PhantomData<I>,
    o: core::marker::PhantomData<O>,
    e: core::marker::PhantomData<E>,
}

impl<F, I, O, E> Recognize<F, I, O, E>
where
    F: Parser<I, O, E>,
    I: Stream,
{
    #[inline(always)]
    pub(crate) fn new(parser: F) -> Self {
        Self {
            parser,
            i: Default::default(),
            o: Default::default(),
            e: Default::default(),
        }
    }
}

impl<I, O, E, F> Parser<I, <I as Stream>::Slice, E> for Recognize<F, I, O, E>
where
    F: Parser<I, O, E>,
    I: Stream,
{
    #[inline]
    fn parse_next(&mut self, input: &mut I) -> PResult<<I as Stream>::Slice, E> {
        let checkpoint = input.checkpoint();
        match (self.parser).parse_next(input) {
            Ok(_) => {
                let offset = input.offset_from(&checkpoint);
                input.reset(checkpoint);
                let recognized = input.next_slice(offset);
                Ok(recognized)
            }
            Err(e) => Err(e),
        }
    }
}

/// Implementation of [`Parser::with_recognized`]
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct WithRecognized<F, I, O, E>
where
    F: Parser<I, O, E>,
    I: Stream,
{
    parser: F,
    i: core::marker::PhantomData<I>,
    o: core::marker::PhantomData<O>,
    e: core::marker::PhantomData<E>,
}

impl<F, I, O, E> WithRecognized<F, I, O, E>
where
    F: Parser<I, O, E>,
    I: Stream,
{
    #[inline(always)]
    pub(crate) fn new(parser: F) -> Self {
        Self {
            parser,
            i: Default::default(),
            o: Default::default(),
            e: Default::default(),
        }
    }
}

impl<F, I, O, E> Parser<I, (O, <I as Stream>::Slice), E> for WithRecognized<F, I, O, E>
where
    F: Parser<I, O, E>,
    I: Stream,
{
    #[inline]
    fn parse_next(&mut self, input: &mut I) -> PResult<(O, <I as Stream>::Slice), E> {
        let checkpoint = input.checkpoint();
        match (self.parser).parse_next(input) {
            Ok(result) => {
                let offset = input.offset_from(&checkpoint);
                input.reset(checkpoint);
                let recognized = input.next_slice(offset);
                Ok((result, recognized))
            }
            Err(e) => Err(e),
        }
    }
}

/// Implementation of [`Parser::span`]
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct Span<F, I, O, E>
where
    F: Parser<I, O, E>,
    I: Stream + Location,
{
    parser: F,
    i: core::marker::PhantomData<I>,
    o: core::marker::PhantomData<O>,
    e: core::marker::PhantomData<E>,
}

impl<F, I, O, E> Span<F, I, O, E>
where
    F: Parser<I, O, E>,
    I: Stream + Location,
{
    #[inline(always)]
    pub(crate) fn new(parser: F) -> Self {
        Self {
            parser,
            i: Default::default(),
            o: Default::default(),
            e: Default::default(),
        }
    }
}

impl<I, O, E, F> Parser<I, Range<usize>, E> for Span<F, I, O, E>
where
    F: Parser<I, O, E>,
    I: Stream + Location,
{
    #[inline]
    fn parse_next(&mut self, input: &mut I) -> PResult<Range<usize>, E> {
        let start = input.location();
        self.parser.parse_next(input).map(move |_| {
            let end = input.location();
            start..end
        })
    }
}

/// Implementation of [`Parser::with_span`]
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct WithSpan<F, I, O, E>
where
    F: Parser<I, O, E>,
    I: Stream + Location,
{
    parser: F,
    i: core::marker::PhantomData<I>,
    o: core::marker::PhantomData<O>,
    e: core::marker::PhantomData<E>,
}

impl<F, I, O, E> WithSpan<F, I, O, E>
where
    F: Parser<I, O, E>,
    I: Stream + Location,
{
    #[inline(always)]
    pub(crate) fn new(parser: F) -> Self {
        Self {
            parser,
            i: Default::default(),
            o: Default::default(),
            e: Default::default(),
        }
    }
}

impl<F, I, O, E> Parser<I, (O, Range<usize>), E> for WithSpan<F, I, O, E>
where
    F: Parser<I, O, E>,
    I: Stream + Location,
{
    #[inline]
    fn parse_next(&mut self, input: &mut I) -> PResult<(O, Range<usize>), E> {
        let start = input.location();
        self.parser.parse_next(input).map(move |output| {
            let end = input.location();
            (output, (start..end))
        })
    }
}

/// Implementation of [`Parser::output_into`]
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct OutputInto<F, I, O, O2, E>
where
    F: Parser<I, O, E>,
    O: Into<O2>,
{
    parser: F,
    i: core::marker::PhantomData<I>,
    o: core::marker::PhantomData<O>,
    o2: core::marker::PhantomData<O2>,
    e: core::marker::PhantomData<E>,
}

impl<F, I, O, O2, E> OutputInto<F, I, O, O2, E>
where
    F: Parser<I, O, E>,
    O: Into<O2>,
{
    #[inline(always)]
    pub(crate) fn new(parser: F) -> Self {
        Self {
            parser,
            i: Default::default(),
            o: Default::default(),
            o2: Default::default(),
            e: Default::default(),
        }
    }
}

impl<F, I, O, O2, E> Parser<I, O2, E> for OutputInto<F, I, O, O2, E>
where
    F: Parser<I, O, E>,
    O: Into<O2>,
{
    #[inline]
    fn parse_next(&mut self, i: &mut I) -> PResult<O2, E> {
        self.parser.parse_next(i).map(|o| o.into())
    }
}

/// Implementation of [`Parser::err_into`]
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct ErrInto<F, I, O, E, E2>
where
    F: Parser<I, O, E>,
    E: Into<E2>,
{
    parser: F,
    i: core::marker::PhantomData<I>,
    o: core::marker::PhantomData<O>,
    e: core::marker::PhantomData<E>,
    e2: core::marker::PhantomData<E2>,
}

impl<F, I, O, E, E2> ErrInto<F, I, O, E, E2>
where
    F: Parser<I, O, E>,
    E: Into<E2>,
{
    #[inline(always)]
    pub(crate) fn new(parser: F) -> Self {
        Self {
            parser,
            i: Default::default(),
            o: Default::default(),
            e: Default::default(),
            e2: Default::default(),
        }
    }
}

impl<F, I, O, E, E2> Parser<I, O, E2> for ErrInto<F, I, O, E, E2>
where
    F: Parser<I, O, E>,
    E: Into<E2>,
{
    #[inline]
    fn parse_next(&mut self, i: &mut I) -> PResult<O, E2> {
        match self.parser.parse_next(i) {
            Ok(ok) => Ok(ok),
            Err(ErrMode::Backtrack(e)) => Err(ErrMode::Backtrack(e.into())),
            Err(ErrMode::Cut(e)) => Err(ErrMode::Cut(e.into())),
            Err(ErrMode::Incomplete(e)) => Err(ErrMode::Incomplete(e)),
        }
    }
}

/// Implementation of [`Parser::context`]
#[cfg_attr(nightly, warn(rustdoc::missing_doc_code_examples))]
pub struct Context<F, I, O, E, C>
where
    F: Parser<I, O, E>,
    I: Stream,
    E: AddContext<I, C>,
    C: Clone + crate::lib::std::fmt::Debug,
{
    parser: F,
    context: C,
    i: core::marker::PhantomData<I>,
    o: core::marker::PhantomData<O>,
    e: core::marker::PhantomData<E>,
}

impl<F, I, O, E, C> Context<F, I, O, E, C>
where
    F: Parser<I, O, E>,
    I: Stream,
    E: AddContext<I, C>,
    C: Clone + crate::lib::std::fmt::Debug,
{
    #[inline(always)]
    pub(crate) fn new(parser: F, context: C) -> Self {
        Self {
            parser,
            context,
            i: Default::default(),
            o: Default::default(),
            e: Default::default(),
        }
    }
}

impl<F, I, O, E, C> Parser<I, O, E> for Context<F, I, O, E, C>
where
    F: Parser<I, O, E>,
    I: Stream,
    E: AddContext<I, C>,
    C: Clone + crate::lib::std::fmt::Debug,
{
    #[inline]
    fn parse_next(&mut self, i: &mut I) -> PResult<O, E> {
        #[cfg(feature = "debug")]
        let name = format!("context={:?}", self.context);
        #[cfg(not(feature = "debug"))]
        let name = "context";
        trace(name, move |i: &mut I| {
            (self.parser)
                .parse_next(i)
                .map_err(|err| err.add_context(i, self.context.clone()))
        })
        .parse_next(i)
    }
}
