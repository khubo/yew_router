use nom::IResult;
use nom::branch::alt;
use nom::sequence::{tuple, pair};
use crate::parser::RouteParserToken;
use nom::combinator::{map, opt};
use nom::multi::{many1, many0};
use nom::bytes::complete::tag;
use crate::parser::core::{capture, match_specific};
use nom::error::{VerboseError, context};
use crate::parser::util::optional_matches;

/// * /
/// * /item
/// * /item/item
/// * /item/item/item
/// * /item(/item)
/// * /item(/item)(/item) and so on
/// * (/item)
/// * (/item)(/item) and so on
pub fn path_parser(i: &str) -> IResult<&str, Vec<RouteParserToken>, VerboseError<&str>> {
    fn inner_path_parser(i: &str) -> IResult<&str, Vec<RouteParserToken>, VerboseError<&str>> {
        context("/ and item",
            map(
            pair(
                separator_token,
                section_matchers
            ),
            |(sep, mut sections)| {
                let mut x = vec![sep];
                x.append(&mut sections);
                x
            }
        ))(i)
    }

    // /item/item/item
    let many_inner_paths = context(
        "many inner paths",
        map(
            many0(inner_path_parser),
            |tokens: Vec<Vec<RouteParserToken>>| {
                tokens.into_iter().flatten().collect::<Vec<_>>()
            }
        )
    );

    // (/item)(/item)(/item)
    let many_optional_inner_paths = context(
        "many optional inner paths",
            many0(optional_matches(inner_path_parser))
    );

    let many_optional_after_concrete_inner = context(
        "many optional after concrete paths",
        map(
            pair(many_inner_paths, many_optional_inner_paths),
            |(mut first, mut second)| {
                first.append(&mut second);
                first
            }
        )
    );

    // accept any number of /thing or just '/
    context("path parser", alt(
        (
            map(
                tuple((many_optional_after_concrete_inner, opt(separator_token))),
                |(mut paths, ending_separator)| {
                    if let Some(end_sep) = ending_separator {
                        paths.push(end_sep)
                    }
                    paths
                }
            ),
            map(separator_token,
                |x| vec![x])
        )
    ))(i)
}


fn separator_token(i: &str) -> IResult<&str, RouteParserToken, VerboseError<&str>> {
    context("/", map(
        tag("/"),
        |_| RouteParserToken::Separator
    ))(i)
}


pub fn section_matchers(i: &str) -> IResult<&str, Vec<RouteParserToken>, VerboseError<&str>> {

    let (i, token): (&str, RouteParserToken) = context("section matchers", alt((match_specific, capture)))(i)?;
    let tokens = vec![token];

    /// You can't have two matching sections in a row, because there is nothing to indicate when
    /// one ends and the other begins.
    /// This function collects possible section matchers and prevents them auto-glob matchers
    /// from residing next to each other.
    fn match_next_section_matchers(i: &str, mut tokens: Vec<RouteParserToken>) -> IResult<&str, Vec<RouteParserToken>, VerboseError<&str>> {
        let token = tokens.last().expect("Must be at least one token.");
        match token {
            RouteParserToken::Match(_) => {
                let (i, t) = opt( capture)(i)?;
                if let Some(new_t) = t {
                    tokens.push(new_t);
                    match_next_section_matchers(i, tokens)
                } else {
                    Ok((i, tokens))
                }
            },
            RouteParserToken::Capture(_) => {
                let (i, t) = opt(match_specific)(i)?;
                if let Some(new_t) = t {
                    tokens.push(new_t);
                    match_next_section_matchers(i, tokens)
                } else {
                    Ok((i, tokens))
                }
            },
            _ => unreachable!()
        }
    }

    match_next_section_matchers(i, tokens)
}

#[cfg(test)]
mod test {
    use super::*;
    use nom::combinator::all_consuming;
    use nom::Err;
    use nom::error::ParseError;
    use nom::error::ErrorKind;
    use nom::error::VerboseErrorKind::{Nom, Context};
    use nom::error::ErrorKind::{Alt, Tag};

    #[test]
    fn path_must_start_with_separator() {
        all_consuming(path_parser)("hello").expect_err("Should reject at absence of /");
    }

    #[test]
    fn path_cant_contain_multiple_matches_in_a_row_0() {
        let e = all_consuming(path_parser)("/path{}{match}").expect_err("Should not validate");
        assert_eq!(e, Err::Error(VerboseError::from_error_kind("{match}", ErrorKind::Eof)))
    }

    #[test]
    fn path_cant_contain_multiple_matches_in_a_row_1() {
        let e = all_consuming(path_parser)("/path{match1}{match2}").expect_err("Should not validate");
        assert_eq!(e, Err::Error(VerboseError::from_error_kind("{match2}", ErrorKind::Eof)))
    }

    #[test]
    fn path_cant_contain_multiple_matches_in_a_row_2() {
        let e = all_consuming(path_parser)("/path{}{}").expect_err("Should not validate");
        assert_eq!(e, Err::Error(VerboseError::from_error_kind("{}", ErrorKind::Eof)))
    }


    #[test]
    fn section_matchers_falis_to_match() {
        let e = section_matchers("{aoeu").expect_err("Should not complete");

        let error = VerboseError {
            errors: vec![
                ("", Nom(Tag)), ("{aoeu", Context("capture")), ("{aoeu", Nom(Alt)), ("{aoeu", Context("section matchers"))
            ]
        };
        assert_eq!(e, Err::Error(error));
    }

    #[test]
    fn cant_have_double_slash() {
        all_consuming(path_parser)("//)").expect_err("Should not validate");
    }

    #[test]
    fn option_section() {
        path_parser("/hello(/hello)").expect("Should validate");
    }

    #[test]
    fn option_section_with_trailing_sep() {
        path_parser("/hello(/hello)/").expect("Should validate");
    }


    #[test]
    fn many_option_section() {
        path_parser("/hello(/hello)(/hello)").expect("Should validate");
    }

    #[test]
    fn option_section_can_start_matcher_string() {
        path_parser("(/hello)").expect("Should validate");
    }

    #[test]
    fn cant_alternate_optional_sections() {
        all_consuming(path_parser)("/hello(/hello)/hello").expect_err("Should not validate");
    }
}