use std::ops::Range;

use winnow::prelude::*;

use winnow::combinator::alt;
use winnow::combinator::cut_err;
use winnow::combinator::peek;
use winnow::combinator::repeat;
//use winnow::combinator::seq;
use winnow::Result;
use winnow::error::ContextError;
use winnow::error::StrContext;
use winnow::error::StrContextValue;
use winnow::stream::LocatingSlice;
use winnow::token::none_of;
use winnow::token::one_of;
use winnow::token::take_while;

type In<'a> = LocatingSlice<&'a str>;

// TODO: Interesting tools:
// '0'.parse_next(), "foo".parse_next
//       one_of(('0'..='9', 'a'..='f', 'A'..='F')).parse_next(input)
// use winnow::ascii::hex_digit1;
// dispatch with known prefixes
// empty() can help
// alt(ernatives)
// pub(crate) fn hex_color(input: &mut &str) -> Result<Color> {
//     seq!(Color {
//         _: '#',
//         red: hex_primary,
//         green: hex_primary,
//         blue: hex_primary
//     })
//     .parse_next(input)
// }

fn comment(input: &mut In) -> Result<()> {
    // GRAMMAR: comment -> "//" ( !'\r' !'\n' ANY )*
    // Intentional: Permit missing trailing \r / \n at EOF
    "//".context(StrContext::Label("comment marker"))
        .context(StrContext::Expected(StrContextValue::Description("//")))
        .parse_next(input)?;
    repeat::<_, _, (), _, _>(0.., none_of(['\r', '\n'])).parse_next(input)?;
    Ok(())
}

fn whitespace(input: &mut In) -> Result<()> {
    // GRAMMAR: whitespace -> ( ' ' | '\t' | '\r' | '\n' | comment )*
    repeat::<_, _, (), _, _>(
        0..,
        alt((one_of([' ', '\t', '\r', '\n']).value(()), comment)),
    )
    .parse_next(input)
}

fn word<'s>(input: &mut In<'s>) -> Result<(&'s str, Range<usize>)> {
    // Heavily inspired by https://docs.rs/winnow/latest/winnow/_topic/language/index.html#identifiers
    // GRAMMAR: word -> ( ALPHA | '_' ) ( ALPHA | NUM | '_' )*
    (
        one_of(|c: char| c.is_alpha() || c == '_')
            .context(StrContext::Label("identifier start"))
            .context(StrContext::Expected(StrContextValue::Description("underscore")))
            .context(StrContext::Expected(StrContextValue::Description("any letter"))),
        take_while(0.., |c: char| c.is_alphanum() || c == '_')
    )
    .take()
    .with_span()
    .parse_next(input)
}

const HEX_CHARS: [char; 16 + 6] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f', 'A', 'B', 'C', 'D', 'E', 'F'];

fn underscored<'s>(char_class: &'static [char]) -> impl ModalParser<In<'s>, (), ContextError> {
    (
        '_',
        // If the character after the underscore is not a hexadecimal digit, parsing fails here.
        cut_err(peek(one_of(char_class))),
    )
    .take()
    .value(())
}

fn number_hex(input: &mut In) -> ModalResult<(u16, Range<usize>)> {
    // Assumption: We already *know* and expect that what follows *must* be a hexadecimal number.
    // GRAMMAR: HEXDIGIT -> '0'..'9' | 'a'..'f' | 'A'..'F'
    // GRAMMAR: number_hex -> HEXDIGIT ( HEXDIGIT | ( '_' &HEXDIGIT ) )*
    // GRAMMAR:   # i.e. underscore is only permitted at most once in a row, and cannot be at either start or end.
    peek(one_of(HEX_CHARS))
        .context(StrContext::Label("hexadecimal number"))
        .context(StrContext::Expected(StrContextValue::Description("hexadecimal digit (i.e. 0-9, a-f, A-F)")))
        .parse_next(input)
        ?;
    repeat::<_, _, String, _, _>(
        1..,
        alt((
            one_of(HEX_CHARS).take(),
            underscored(&HEX_CHARS).value(""),
        )),
    )
    .verify(|s: &String| s.len() <= 4)
    .try_map(|s: String| u16::from_str_radix(&s, 16))
    .context(StrContext::Label("hexadecimal number with at most 4 hexits"))
    .context(StrContext::Expected(StrContextValue::Description("hexit (i.e. 0-9, a-f, A-F)")))
    .context(StrContext::Expected(StrContextValue::Description("underscore (followed by a hexit)")))
    .with_span()
    .parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment_minimal() {
        let mut input = In::new("//");
        let output = comment(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(*input, "");
    }

    #[test]
    fn test_comment_longer() {
        let mut input = In::new("// hello // world!! \\r\\n lol still the same line");
        let output = comment(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(*input, "");
    }

    #[test]
    fn test_comment_tail() {
        let mut input = In::new("// hi\r\n");
        let output = comment(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(*input, "\r\n");
    }

    #[test]
    fn test_comment_incomplete() {
        let input = In::new("/ b");
        let actual_err = comment.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 0..1);
        let expected_err = "/ b\n^\ninvalid comment marker\nexpected //";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_comment_incomplete_minimal() {
        let input = In::new("/");
        let actual_err = comment.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 0..1);
        let expected_err = "/\n^\ninvalid comment marker\nexpected //";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_comment_incomplete_empty() {
        let input = In::new("");
        let actual_err = comment.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 0..0);
        let expected_err = "\n^\ninvalid comment marker\nexpected //";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_whitespace_none() {
        let mut input = In::new("");
        let output = whitespace(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(*input, "");
    }

    #[test]
    fn test_whitespace_pseudofail() {
        let mut input = In::new("x");
        let output = whitespace(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(*input, "x");
    }

    #[test]
    fn test_whitespace_minimal_space() {
        let mut input = In::new(" x");
        let output = whitespace(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(*input, "x");
    }

    #[test]
    fn test_whitespace_minimal_t() {
        let mut input = In::new("\tx");
        let output = whitespace(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(*input, "x");
    }

    #[test]
    fn test_whitespace_minimal_r() {
        let mut input = In::new("\rx");
        let output = whitespace(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(*input, "x");
    }

    #[test]
    fn test_whitespace_minimal_n() {
        let mut input = In::new("\nx");
        let output = whitespace(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(*input, "x");
    }

    #[test]
    fn test_whitespace_minimal_crlf() {
        let mut input = In::new("\r\nx");
        let output = whitespace(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(*input, "x");
    }

    #[test]
    fn test_whitespace_comment() {
        let mut input = In::new("// holy crap\r\nx");
        let output = whitespace(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(*input, "x");
    }

    #[test]
    fn test_whitespace_many() {
        let mut input = In::new(" // hello\n\t// worl\nx\rtrail");
        let output = whitespace(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(*input, "x\rtrail");
    }

    #[test]
    fn test_word_none() {
        let input = In::new("");
        let actual_err = word.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 0..0);
        let expected_err = "\n^\ninvalid identifier start\nexpected underscore, any letter";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_word_digit() {
        let input = In::new("5");
        let actual_err = word.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 0..1);
        let expected_err = "5\n^\ninvalid identifier start\nexpected underscore, any letter";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_word_minimal_alpha() {
        let mut input = In::new("a b c");
        let output = word(&mut input).expect("parse failed");
        assert_eq!(output, ("a", 0..1));
        assert_eq!(*input, " b c");
    }

    #[test]
    fn test_word_minimal_underscore() {
        let mut input = In::new("_ _ _");
        let output = word(&mut input).expect("parse failed");
        assert_eq!(output, ("_", 0..1));
        assert_eq!(*input, " _ _");
    }

    #[test]
    fn test_word_minimal_alpha_digit() {
        let mut input = In::new("r7+r8");
        let output = word(&mut input).expect("parse failed");
        assert_eq!(output, ("r7", 0..2));
        assert_eq!(*input, "+r8");
    }

    #[test]
    fn test_word_short() {
        let mut input = In::new("hello world");
        let output = word(&mut input).expect("parse failed");
        assert_eq!(output, ("hello", 0..5));
        assert_eq!(*input, " world");
    }

    #[test]
    fn test_word_complex() {
        let mut input = In::new("ComplicatedThing1234_XXXZZ.lol()");
        let output = word(&mut input).expect("parse failed");
        assert_eq!(output, ("ComplicatedThing1234_XXXZZ", 0..26));
        assert_eq!(*input, ".lol()");
    }

    #[test]
    fn test_word_space_word() {
        let mut input = In::new("hello world");
        let output = word(&mut input).expect("parse failed");
        assert_eq!(output, ("hello", 0..5));
        assert_eq!(*input, " world");
        let output = whitespace(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(*input, "world");
        let output = word(&mut input).expect("parse failed");
        assert_eq!(output, ("world", 6..11));
        assert_eq!(*input, "");
    }

    #[test]
    fn test_numhex_minimal_digit() {
        let mut input = In::new("9yooo");
        let output = number_hex(&mut input).expect("parse failed");
        assert_eq!(output, (9, 0..1));
        assert_eq!(*input, "yooo");
    }

    #[test]
    fn test_numhex_minimal_hexit() {
        let mut input = In::new("ayooo");
        let output = number_hex(&mut input).expect("parse failed");
        assert_eq!(output, (10, 0..1));
        assert_eq!(*input, "yooo");
    }

    #[test]
    fn test_numhex_max() {
        let mut input = In::new("ffffun!");
        let output = number_hex(&mut input).expect("parse failed");
        assert_eq!(output, (65535, 0..4));
        assert_eq!(*input, "un!");
    }

    #[test]
    fn test_numhex_several_zero() {
        let mut input = In::new("00");
        let output = number_hex(&mut input).expect("parse failed");
        assert_eq!(output, (0, 0..2));
        assert_eq!(*input, "");
    }

    #[test]
    fn test_numhex_too_long() {
        let input = In::new("12345");
        let actual_err = number_hex.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 0..1);
        let expected_err = "12345\n^\ninvalid hexadecimal number with at most 4 hexits\nexpected hexit (i.e. 0-9, a-f, A-F), underscore (followed by a hexit)";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_numhex_too_long_zero() {
        let input = In::new("00000");
        let actual_err = number_hex.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 0..1);
        let expected_err = "00000\n^\ninvalid hexadecimal number with at most 4 hexits\nexpected hexit (i.e. 0-9, a-f, A-F), underscore (followed by a hexit)";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_numhex_empty() {
        let input = In::new("");
        let actual_err = number_hex.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 0..0);
        let expected_err = "\n^\ninvalid hexadecimal number\nexpected hexadecimal digit (i.e. 0-9, a-f, A-F)";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_numhex_invalid() {
        let input = In::new("g");
        let actual_err = number_hex.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 0..1);
        let expected_err = "g\n^\ninvalid hexadecimal number\nexpected hexadecimal digit (i.e. 0-9, a-f, A-F)";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_numhex_invalid_tail() {
        let input = In::new("efgh");
        let actual_err = number_hex.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 2..3);
        // Message is only bad because it matches against eof() under the hood, so can't inject anything
        let expected_err = "efgh\n  ^\n";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_numhex_max_underscores() {
        let mut input = In::new("f_a_c_e+");
        let output = number_hex(&mut input).expect("parse failed");
        assert_eq!(output, (0xFACE, 0..7));
        assert_eq!(*input, "+");
    }

    #[test]
    fn test_numhex_double_underscore() {
        let input = In::new("12__3 * 9");
        let actual_err = number_hex.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 3..4);
        // Message is only bad because it matches against eof() under the hood, so can't inject anything
        let expected_err = "12__3 * 9\n   ^\ninvalid hexadecimal number with at most 4 hexits\nexpected hexit (i.e. 0-9, a-f, A-F), underscore (followed by a hexit)";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_numhex_leading_underscore() {
        let input = In::new("_eee");
        let actual_err = number_hex.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 0..1);
        // Message is only bad because it matches against eof() under the hood, so can't inject anything
        let expected_err = "_eee\n^\ninvalid hexadecimal number\nexpected hexadecimal digit (i.e. 0-9, a-f, A-F)";
        assert_eq!(actual_err.to_string(), expected_err);
    }

}
