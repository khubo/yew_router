//! Core functions for working with the route parser.
use nom::IResult;
use nom::combinator::{peek, map};
use nom::character::is_digit;
use nom::error::{ErrorKind, context, VerboseError};
use nom::bytes::complete::{is_not, take, tag};
use nom::branch::alt;
use crate::parser::RouteParserToken;
use nom::sequence::{preceded, separated_pair, delimited};
use nom::character::complete::digit1;
use crate::parser::CaptureVariant;
use crate::parser::CaptureOrMatch;
use nom::error::ParseError;


/// Captures a string up to the point where a character not possible to be present in Rust's identifier is encountered.
/// It prevents the first character from being a digit.
pub fn valid_ident_characters(i: &str) -> IResult<&str, &str, VerboseError<&str>> {
    const INVALID_CHARACTERS: &str = " -*/+#?&^@~`;,.|\\{}[]()=\t\n";
    context(
        "valid ident",
        |i: &str| {
            let (i, next) = peek(take(1usize))(i)?; // Look at the first character
            if is_digit(next.bytes().next().unwrap()) {
                //        return Err(nom::Err(VerboseError::from("Digits not allowed"))) // Digits not allowed
                return Err(nom::Err::Error(VerboseError::from_error_kind(i, ErrorKind::Digit)))
            } else {
                is_not(INVALID_CHARACTERS)(i)
            }
        })(i)
}

/// Captures groups of characters that will need to be matched exactly later.
pub fn match_specific(i: &str) -> IResult<&str, RouteParserToken, VerboseError<&str>> {
    context("match", map(
        valid_ident_characters,
        |ident| RouteParserToken::Match(ident.to_string())
    ))(i)
}


/// Matches any of the capture variants
///
/// * {}
/// * {*}
/// * {5}
/// * {name}
/// * {*:name}
/// * {5:name}
pub fn capture(i: &str) -> IResult<&str, RouteParserToken, VerboseError<&str>> {
    let capture_variants = alt(
        (
            map(peek(tag("}")), |_| CaptureVariant::Unnamed), // just empty {}
            map(preceded(tag("*:"), valid_ident_characters), |s| CaptureVariant::ManyNamed(s.to_string())),
            map(tag("*"), |_| CaptureVariant::ManyUnnamed),
            map(valid_ident_characters, |s| CaptureVariant::Named(s.to_string())),
            map(separated_pair(digit1, tag(":"), valid_ident_characters), |(n, s)| CaptureVariant::NumberedNamed {sections: n.parse().expect("Should parse digits"), name: s.to_string()}),
            map(digit1, |num: &str| CaptureVariant::NumberedUnnamed {sections: num.parse().expect("should parse digits" )})
        )
    );

    context("capture", map(
        delimited(tag("{"), capture_variants, tag("}")),
        RouteParserToken::Capture
    ))(i)
}

/// Matches either "item" or "{capture}"
/// It returns a subset enum of Token.
pub fn capture_or_match(i: &str) -> IResult<&str, CaptureOrMatch, VerboseError<&str>> {
    let (i, token) = context("capture or match", alt((capture, match_specific)))(i)?;
    let token = match token {
        RouteParserToken::Capture(variant) => CaptureOrMatch::Capture(variant),
        RouteParserToken::Match(m) => CaptureOrMatch::Match(m),
        _ => unreachable!("Only should handle captures and matches")
    };
    Ok((i, token))
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn match_any() {
        let cap = capture("{}").expect("Should match").1;
        assert_eq!(cap, RouteParserToken::Capture(CaptureVariant::Unnamed));
    }

    #[test]
    fn capture_named_test() {
        let cap = capture("{hellothere}").unwrap();
        assert_eq!(cap, ("", RouteParserToken::Capture (CaptureVariant::Named("hellothere".to_string()))));
    }

    #[test]
    fn capture_many_unnamed_test() {
        let cap = capture("{*}").unwrap();
        assert_eq!(cap, ("", RouteParserToken::Capture (CaptureVariant::ManyUnnamed)));
    }

    #[test]
    fn capture_unnamed_test() {
        let cap = capture("{}").unwrap();
        assert_eq!(cap, ("", RouteParserToken::Capture (CaptureVariant::Unnamed)));
    }

    #[test]
    fn capture_numbered_unnamed_test() {
        let cap = capture("{5}").unwrap();
        assert_eq!(cap, ("", RouteParserToken::Capture (CaptureVariant::NumberedUnnamed {sections: 5})));
    }

    #[test]
    fn capture_numbered_named_test() {
        let cap = capture("{5:name}").unwrap();
        assert_eq!(cap, ("", RouteParserToken::Capture (CaptureVariant::NumberedNamed{sections: 5, name: "name".to_string()})));
    }


    #[test]
    fn capture_many_named() {
        let cap = capture("{*:name}").unwrap();
        assert_eq!(cap, ("", RouteParserToken::Capture (CaptureVariant::ManyNamed("name".to_string()))));
    }

    #[test]
    fn rejects_invalid_ident() {
        valid_ident_characters("+-Hello").expect_err("Should reject at +");
    }

    #[test]
    fn accepts_valid_ident() {
        valid_ident_characters("Hello").expect("Should accept");
    }

    #[test]
    fn capture_consumes() {
        capture("{aoeu").expect_err("Should not complete");
    }

}