use winnow::{
    ascii::line_ending, combinator::repeat, prelude::*, stream::Partial, token::take_while,
};

pub type Stream<'i> = Partial<&'i [u8]>;

#[rustfmt::skip]
#[derive(Debug)]
#[allow(dead_code)]
pub struct Request<'a> {
  method:  &'a [u8],
  uri:     &'a [u8],
  version: &'a [u8],
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Header<'a> {
    name: &'a [u8],
    value: Vec<&'a [u8]>,
}

pub fn parse(data: &[u8]) -> Option<Vec<(Request<'_>, Vec<Header<'_>>)>> {
    let mut buf = Partial::new(data);
    let mut v = Vec::new();
    loop {
        match request(&mut buf) {
            Ok(r) => {
                v.push(r);

                if buf.is_empty() {
                    //println!("{}", i);
                    break;
                }
            }
            Err(e) => {
                println!("error: {:?}", e);
                return None;
            }
        }
    }

    Some(v)
}

fn request<'s>(input: &mut Stream<'s>) -> PResult<(Request<'s>, Vec<Header<'s>>)> {
    let req = request_line(input)?;
    let h = repeat(1.., message_header).parse_next(input)?;
    let _ = line_ending.parse_next(input)?;

    Ok((req, h))
}

fn request_line<'s>(input: &mut Stream<'s>) -> PResult<Request<'s>> {
    let method = take_while(1.., is_token).parse_next(input)?;
    let _ = take_while(1.., is_space).parse_next(input)?;
    let uri = take_while(1.., is_not_space).parse_next(input)?;
    let _ = take_while(1.., is_space).parse_next(input)?;
    let version = http_version(input)?;
    let _ = line_ending.parse_next(input)?;

    Ok(Request {
        method,
        uri,
        version,
    })
}

fn http_version<'s>(input: &mut Stream<'s>) -> PResult<&'s [u8]> {
    let _ = "HTTP/".parse_next(input)?;
    let version = take_while(1.., is_version).parse_next(input)?;

    Ok(version)
}

fn message_header_value<'s>(input: &mut Stream<'s>) -> PResult<&'s [u8]> {
    let _ = take_while(1.., is_horizontal_space).parse_next(input)?;
    let data = take_while(1.., not_line_ending).parse_next(input)?;
    let _ = line_ending.parse_next(input)?;

    Ok(data)
}

fn message_header<'s>(input: &mut Stream<'s>) -> PResult<Header<'s>> {
    let name = take_while(1.., is_token).parse_next(input)?;
    let _ = ':'.parse_next(input)?;
    let value = repeat(1.., message_header_value).parse_next(input)?;

    Ok(Header { name, value })
}

#[rustfmt::skip]
#[allow(clippy::match_same_arms)]
#[allow(clippy::match_like_matches_macro)]
fn is_token(c: u8) -> bool {
  match c {
    128..=255 => false,
    0..=31    => false,
    b'('      => false,
    b')'      => false,
    b'<'      => false,
    b'>'      => false,
    b'@'      => false,
    b','      => false,
    b';'      => false,
    b':'      => false,
    b'\\'     => false,
    b'"'      => false,
    b'/'      => false,
    b'['      => false,
    b']'      => false,
    b'?'      => false,
    b'='      => false,
    b'{'      => false,
    b'}'      => false,
    b' '      => false,
    _         => true,
  }
}

fn is_version(c: u8) -> bool {
    c.is_ascii_digit() || c == b'.'
}

fn not_line_ending(c: u8) -> bool {
    c != b'\r' && c != b'\n'
}

fn is_space(c: u8) -> bool {
    c == b' '
}

fn is_not_space(c: u8) -> bool {
    c != b' '
}

fn is_horizontal_space(c: u8) -> bool {
    c == b' ' || c == b'\t'
}
