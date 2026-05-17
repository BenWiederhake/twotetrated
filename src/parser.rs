use std::fmt::Debug;
use std::hash::Hash;
use std::ops::Range;

use winnow::prelude::*;

use winnow::combinator::alt;
use winnow::combinator::cut_err;
use winnow::combinator::delimited;
use winnow::combinator::fail;
use winnow::combinator::peek;
use winnow::combinator::preceded;
use winnow::combinator::impls::WithSpan;
use winnow::combinator::opt;
use winnow::combinator::repeat;
//use winnow::combinator::seq;
use winnow::combinator::terminated;
use winnow::Result;
use winnow::error::ContextError;
use winnow::error::StrContext;
use winnow::error::StrContextValue;
use winnow::stream::LocatingSlice;
use winnow::stream::Stateful;
use winnow::token::none_of;
use winnow::token::one_of;
use winnow::token::take_while;

use crate::ast::BracedBody;
use crate::ast::Expression;
use crate::ast::FileSpan;
use crate::ast::FunctionItem;
use crate::ast::Located;
use crate::ast::LocatedBracedBody;
use crate::ast::LocatedExpression;
use crate::ast::LocatedFunctionItem;
use crate::ast::LocatedStatement;
use crate::ast::LocatedToken;
use crate::ast::LocatedType;
use crate::ast::LocatedVariableDecl;
use crate::ast::Statement;

type In<'is> = Stateful<LocatingSlice<&'is str>, &'is str>;

fn stream_from<'is>(code: &'is str, filename: &'is str) -> In<'is> {
    In {
        input: LocatingSlice::new(code),
        state: filename,
    }
}

struct WithFileSpan<'is, F, O, E>
where
    F: Parser<In<'is>, (O, Range<usize>), E>,
{
    parser: F, // WithSpan
    marker: core::marker::PhantomData<(In<'is>, O, E)>,
}

impl<'is, F, O, E> Parser<In<'is>, Located<'is, O>, E> for WithFileSpan<'is, F, O, E>
where
    F: Parser<In<'is>, (O, Range<usize>), E>,
    O: Debug + Clone + PartialEq + Eq + Hash,
{
    // TODO: Decide whether to mark as #[inline] (it's inline in WithSpan)
    fn parse_next(&mut self, input: &mut In<'is>) -> Result<Located<'is, O>, E> {
        //let start = input.current_token_start();
        self.parser.parse_next(input).map(move |output_tuple| {
            let (output, char_span) = output_tuple;
            Located::new(
                output,
                FileSpan {
                    source: input.state,
                    chars: char_span,
                },
            )
        })
    }
}

trait WithFileSpanExt<'is, O, E, Base>
where
    Base: Sized + Parser<In<'is>, O, E>,
{
    fn with_file_span(self) -> WithFileSpan<'is, WithSpan<Base, In<'is>, O, E>, O, E>;
}

impl<'is, O, E, T: Sized + Parser<In<'is>, O, E>> WithFileSpanExt<'is, O, E, T> for T {
    fn with_file_span(self) -> WithFileSpan<'is, WithSpan<Self, In<'is>, O, E>, O, E> {
        WithFileSpan { parser: self.with_span(), marker: Default::default() }
    }
}

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

fn parse_comment(input: &mut In) -> ModalResult<()> {
    // GRAMMAR: comment -> "//" ( !'\r' !'\n' ANY )*
    // Intentional: Permit missing trailing \r / \n at EOF
    "//".context(StrContext::Label("comment marker"))
        .context(StrContext::Expected(StrContextValue::Description("//")))
        .parse_next(input)?;
    repeat::<_, _, (), _, _>(0.., none_of(['\r', '\n'])).parse_next(input)?;
    Ok(())
}

fn parse_whitespace(input: &mut In) -> ModalResult<()> {
    // GRAMMAR: whitespace -> ( ' ' | '\t' | '\r' | '\n' | comment )*
    repeat::<_, _, (), _, _>(
        0..,
        alt((one_of([' ', '\t', '\r', '\n']).value(()), parse_comment)),
    )
    .parse_next(input)
}

fn parse_word<'s>(input: &mut In<'s>) -> ModalResult<LocatedToken<'s>> {
    // Heavily inspired by https://docs.rs/winnow/latest/winnow/_topic/language/index.html#identifiers
    // GRAMMAR: word -> ( ALPHA | '_' ) ( ALPHA | NUM | '_' )*
    (
        one_of(|c: char| c.is_alpha() || c == '_')
            .context(StrContext::Label("identifier start"))
            .context(StrContext::Expected(StrContextValue::Description(
                "underscore",
            )))
            .context(StrContext::Expected(StrContextValue::Description(
                "any letter",
            ))),
        take_while(0.., |c: char| c.is_alphanum() || c == '_'),
    )
        .take()
        .with_file_span()
        .parse_next(input)
}

const HEX_CHARS: [char; 16 + 6] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', // decimals
    'a', 'b', 'c', 'd', 'e', 'f', // lowercase
    'A', 'B', 'C', 'D', 'E', 'F', // uppercase
];

fn underscored<'s>(char_class: &'static [char]) -> impl ModalParser<In<'s>, (), ContextError> {
    (
        '_',
        // If the character after the underscore is not a hexadecimal digit, parsing fails here.
        cut_err(peek(one_of(char_class))),
    )
        .take()
        .value(())
}

fn parse_number_hex(input: &mut In) -> ModalResult<u16> {
    // Assumption: We already *know* and expect that what follows *must* be a hexadecimal number.
    // GRAMMAR: HEXDIGIT -> '0'..'9' | 'a'..'f' | 'A'..'F'
    // GRAMMAR: number_hex -> HEXDIGIT ( HEXDIGIT | ( '_' &HEXDIGIT ) )*
    // GRAMMAR:   # i.e. underscore is only permitted at most once in a row, and cannot be at either start or end.
    peek(one_of(HEX_CHARS))
        .context(StrContext::Label("hexadecimal number"))
        .context(StrContext::Expected(StrContextValue::Description(
            "hexadecimal digit (i.e. 0-9, a-f, A-F)",
        )))
        .parse_next(input)?;
    repeat::<_, _, String, _, _>(
        1..,
        alt((one_of(HEX_CHARS).take(), underscored(&HEX_CHARS).value(""))),
    )
    .verify(|s: &String| s.len() <= 4)
    .try_map(|s: String| u16::from_str_radix(&s, 16))
    .context(StrContext::Label(
        "hexadecimal number with at most 4 hexits",
    ))
    .context(StrContext::Expected(StrContextValue::Description(
        "hexit (i.e. 0-9, a-f, A-F)",
    )))
    .context(StrContext::Expected(StrContextValue::Description(
        "underscore (followed by a hexit)",
    )))
    .parse_next(input)
}

fn parse_number<'s>(input: &mut In<'s>) -> ModalResult<Located<'s, u16>> {
    // GRAMMAR: number -> "0x" number_hex  // FIXME: More prefixes
    // TODO: Use dispatch and cut to select prefix
    preceded(
        "0x".context(StrContext::Label("number prefix"))
            .context(StrContext::Expected(StrContextValue::Description(
                "only 0x (FIXME)",
            ))),
        parse_number_hex,
    )
    .with_file_span()
    .parse_next(input)
}

fn parse_expression<'s>(input: &mut In<'s>) -> ModalResult<LocatedExpression<'s>> {
    // GRAMMAR: expression -> number  // (literal) FIXME: So many more types of expression!
    let located_number = parse_number(input)?;
    Ok(LocatedExpression::new(
        Expression::Literal(located_number.value),
        located_number.span,
    ))
}

fn parse_stmt_yield<'s>(input: &mut In<'s>) -> ModalResult<LocatedStatement<'s>> {
    // GRAMMAR: statement -> "yield" ws expression ws ";"  // (yield) FIXME: So many more types of statement!
    let yield_token = parse_word.verify(|located_word| located_word.value == "yield").parse_next(input)?;
    let expr = cut_err(delimited(
        parse_whitespace,
        parse_expression,
        ";",
    ))
        .parse_next(input)?;
    Ok(LocatedStatement::new(
        Statement::Yield(expr),
        yield_token.span,
    ))
}

fn parse_statement<'s>(input: &mut In<'s>) -> ModalResult<LocatedStatement<'s>> {
    // Use alt() or something?
    parse_stmt_yield(input)
}

fn parse_braced_body<'s>(input: &mut In<'s>) -> ModalResult<LocatedBracedBody<'s>> {
    // GRAMMAR: braced_body -> "{" ws (expression ws)* "}"
    // TODO: Give context?
    let opening_brace = "{".with_file_span().parse_next(input)?;
    let statements = cut_err(delimited(
        parse_whitespace,
        repeat(0.., terminated(parse_statement, parse_whitespace)),
        "}",
    ))
        .parse_next(input)?;
    Ok(LocatedBracedBody::new(
        BracedBody(statements),
        opening_brace.span,
    ))
}

fn parse_identifier<'s>(input: &mut In<'s>) -> ModalResult<LocatedToken<'s>> {
    // GRAMMAR: identifier -> !reserved_word word
    let word = parse_word(input)?;
    // FIXME: yolo! check against reserved words
    // (DO NOT CUT in case of (mis)match!)
    Ok(word)
}

fn parse_typed_argument_list<'s>(input: &mut In<'s>) -> ModalResult<Vec<LocatedVariableDecl<'s>>> {
    // GRAMMAR: typed_argument_list -> () // FIXME: Everything!
    Ok(Vec::new())
}

fn parse_return_declaration<'s>(input: &mut In<'s>) -> ModalResult<LocatedType<'s>> {
    // GRAMMAR: return_decl -> "->" ws typename // FIXME: Everything!
    // FIXME: Not implemented, cut after "->"
    fail(input)
}

fn parse_function_item<'s>(input: &mut In<'s>) -> ModalResult<LocatedFunctionItem<'s>> {
    // GRAMMAR: function_item -> "function" ws identifier ws "(" ws typed_argument_list ")" ws return_decl? ws braced_body
    // TODO: Give context?
    let (function, (name, argument_list, return_declaration, body)) =
        (
            "function".with_file_span(),
            cut_err((
                delimited(parse_whitespace, parse_identifier, parse_whitespace),
                delimited(("(", parse_whitespace), parse_typed_argument_list, (parse_whitespace, ")")),
                preceded(parse_whitespace, opt(parse_return_declaration)),
                preceded(parse_whitespace, parse_braced_body),
            )),
        )
            .parse_next(input)?;
    Ok(LocatedFunctionItem::new(
        FunctionItem {
            name,
            argument_list,
            return_declaration,
            body,
        },
        function.span,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment_minimal() {
        let mut input = stream_from("//", "<input>");
        let output = parse_comment(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(**input, "");
    }

    #[test]
    fn test_comment_longer() {
        let mut input = stream_from(
            "// hello // world!! \\r\\n lol still the same line",
            "<input>",
        );
        let output = parse_comment(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(**input, "");
    }

    #[test]
    fn test_comment_tail() {
        let mut input = stream_from("// hi\r\n", "<input>");
        let output = parse_comment(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(**input, "\r\n");
    }

    #[test]
    fn test_comment_incomplete() {
        let input = stream_from("/ b", "<input>");
        let actual_err = parse_comment.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 0..1);
        let expected_err = "/ b\n^\ninvalid comment marker\nexpected //";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_comment_incomplete_minimal() {
        let input = stream_from("/", "<input>");
        let actual_err = parse_comment.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 0..1);
        let expected_err = "/\n^\ninvalid comment marker\nexpected //";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_comment_incomplete_empty() {
        let input = stream_from("", "<input>");
        let actual_err = parse_comment.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 0..0);
        let expected_err = "\n^\ninvalid comment marker\nexpected //";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_whitespace_none() {
        let mut input = stream_from("", "<input>");
        let output = parse_whitespace(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(**input, "");
    }

    #[test]
    fn test_whitespace_pseudofail() {
        let mut input = stream_from("x", "<input>");
        let output = parse_whitespace(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(**input, "x");
    }

    #[test]
    fn test_whitespace_minimal_space() {
        let mut input = stream_from(" x", "<input>");
        let output = parse_whitespace(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(**input, "x");
    }

    #[test]
    fn test_whitespace_minimal_t() {
        let mut input = stream_from("\tx", "<input>");
        let output = parse_whitespace(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(**input, "x");
    }

    #[test]
    fn test_whitespace_minimal_r() {
        let mut input = stream_from("\rx", "<input>");
        let output = parse_whitespace(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(**input, "x");
    }

    #[test]
    fn test_whitespace_minimal_n() {
        let mut input = stream_from("\nx", "<input>");
        let output = parse_whitespace(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(**input, "x");
    }

    #[test]
    fn test_whitespace_minimal_crlf() {
        let mut input = stream_from("\r\nx", "<input>");
        let output = parse_whitespace(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(**input, "x");
    }

    #[test]
    fn test_whitespace_comment() {
        let mut input = stream_from("// holy crap\r\nx", "<input>");
        let output = parse_whitespace(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(**input, "x");
    }

    #[test]
    fn test_whitespace_many() {
        let mut input = stream_from(" // hello\n\t// worl\nx\rtrail", "<input>");
        let output = parse_whitespace(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(**input, "x\rtrail");
    }

    #[test]
    fn test_word_none() {
        let input = stream_from("", "<input>");
        let actual_err = parse_word.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 0..0);
        let expected_err = "\n^\ninvalid identifier start\nexpected underscore, any letter";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_word_digit() {
        let input = stream_from("5", "<input>");
        let actual_err = parse_word.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 0..1);
        let expected_err = "5\n^\ninvalid identifier start\nexpected underscore, any letter";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_word_minimal_alpha() {
        let mut input = stream_from("a b c", "<input>");
        let output = parse_word(&mut input).expect("parse failed");
        assert_eq!(output.value, "a");
        assert_eq!(output.span, FileSpan::new("<input>", 0..1));
        assert_eq!(**input, " b c");
    }

    #[test]
    fn test_word_minimal_underscore() {
        let mut input = stream_from("_ _ _", "<input>");
        let output = parse_word(&mut input).expect("parse failed");
        assert_eq!(output.value, "_");
        assert_eq!(output.span, FileSpan::new("<input>", 0..1));
        assert_eq!(**input, " _ _");
    }

    #[test]
    fn test_word_minimal_alpha_digit() {
        let mut input = stream_from("r7+r8", "<input>");
        let output = parse_word(&mut input).expect("parse failed");
        assert_eq!(output.value, "r7");
        assert_eq!(output.span, FileSpan::new("<input>", 0..2));
        assert_eq!(**input, "+r8");
    }

    #[test]
    fn test_word_short() {
        let mut input = stream_from("hello world", "<input>");
        let output = parse_word(&mut input).expect("parse failed");
        assert_eq!(output.value, "hello");
        assert_eq!(output.span, FileSpan::new("<input>", 0..5));
        assert_eq!(**input, " world");
    }

    #[test]
    fn test_word_complex() {
        let mut input = stream_from("ComplicatedThing1234_XXXZZ.lol()", "<input>");
        let output = parse_word(&mut input).expect("parse failed");
        assert_eq!(output.value, "ComplicatedThing1234_XXXZZ");
        assert_eq!(output.span, FileSpan::new("<input>", 0..26));
        assert_eq!(**input, ".lol()");
    }

    #[test]
    fn test_word_space_word() {
        let mut input = stream_from("hello world", "<input>");
        let output = parse_word(&mut input).expect("parse failed");
        assert_eq!(output.value, "hello");
        assert_eq!(output.span, FileSpan::new("<input>", 0..5));
        assert_eq!(**input, " world");
        let output = parse_whitespace(&mut input).expect("parse failed");
        assert_eq!(output, ());
        assert_eq!(**input, "world");
        let output = parse_word(&mut input).expect("parse failed");
        assert_eq!(output.value, "world");
        assert_eq!(output.span, FileSpan::new("<input>", 6..11));
        assert_eq!(**input, "");
    }

    #[test]
    fn test_numhex_minimal_digit() {
        let mut input = stream_from("9yooo", "<input>");
        let output = parse_number_hex(&mut input).expect("parse failed");
        assert_eq!(output, 9);
        assert_eq!(**input, "yooo");
    }

    #[test]
    fn test_numhex_minimal_hexit() {
        let mut input = stream_from("ayooo", "<input>");
        let output = parse_number_hex(&mut input).expect("parse failed");
        assert_eq!(output, 10);
        assert_eq!(**input, "yooo");
    }

    #[test]
    fn test_numhex_max() {
        let mut input = stream_from("ffffun!", "<input>");
        let output = parse_number_hex(&mut input).expect("parse failed");
        assert_eq!(output, 65535);
        assert_eq!(**input, "un!");
    }

    #[test]
    fn test_numhex_several_zero() {
        let mut input = stream_from("00", "<input>");
        let output = parse_number_hex(&mut input).expect("parse failed");
        assert_eq!(output, 0);
        assert_eq!(**input, "");
    }

    #[test]
    fn test_numhex_too_long() {
        let input = stream_from("12345", "<input>");
        let actual_err = parse_number_hex.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 0..1);
        let expected_err = "12345\n^\ninvalid hexadecimal number with at most 4 hexits\nexpected hexit (i.e. 0-9, a-f, A-F), underscore (followed by a hexit)";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_numhex_too_long_zero() {
        let input = stream_from("00000", "<input>");
        let actual_err = parse_number_hex.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 0..1);
        let expected_err = "00000\n^\ninvalid hexadecimal number with at most 4 hexits\nexpected hexit (i.e. 0-9, a-f, A-F), underscore (followed by a hexit)";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_numhex_empty() {
        let input = stream_from("", "<input>");
        let actual_err = parse_number_hex.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 0..0);
        let expected_err =
            "\n^\ninvalid hexadecimal number\nexpected hexadecimal digit (i.e. 0-9, a-f, A-F)";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_numhex_invalid() {
        let input = stream_from("g", "<input>");
        let actual_err = parse_number_hex.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 0..1);
        let expected_err =
            "g\n^\ninvalid hexadecimal number\nexpected hexadecimal digit (i.e. 0-9, a-f, A-F)";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_numhex_invalid_tail() {
        let input = stream_from("efgh", "<input>");
        let actual_err = parse_number_hex.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 2..3);
        // Message is only bad because it matches against eof() under the hood, so can't inject anything
        let expected_err = "efgh\n  ^\n";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_numhex_max_underscores() {
        let mut input = stream_from("f_a_c_e+", "<input>");
        let output = parse_number_hex(&mut input).expect("parse failed");
        assert_eq!(output, 0xFACE);
        assert_eq!(**input, "+");
    }

    #[test]
    fn test_numhex_double_underscore() {
        let input = stream_from("12__3 * 9", "<input>");
        let actual_err = parse_number_hex.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 3..4);
        // Message is only bad because it matches against eof() under the hood, so can't inject anything
        let expected_err = "12__3 * 9\n   ^\ninvalid hexadecimal number with at most 4 hexits\nexpected hexit (i.e. 0-9, a-f, A-F), underscore (followed by a hexit)";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_numhex_leading_underscore() {
        let input = stream_from("_eee", "<input>");
        let actual_err = parse_number_hex.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 0..1);
        // Message is only bad because it matches against eof() under the hood, so can't inject anything
        let expected_err =
            "_eee\n^\ninvalid hexadecimal number\nexpected hexadecimal digit (i.e. 0-9, a-f, A-F)";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_number_hex_minimal() {
        let mut input = stream_from("0x1;", "<input>");
        let output = parse_number(&mut input).expect("parse failed");
        assert_eq!(output.value, 1);
        assert_eq!(output.span, FileSpan::new("<input>", 0..3));
        assert_eq!(**input, ";");
    }

    #[test]
    fn test_number_invalid_prefix() {
        let input = stream_from("0p4", "<input>");
        let actual_err = parse_number.parse(input).expect_err("parse succeeded?!");
        assert_eq!(actual_err.char_span(), 0..1);
        let expected_err = "0p4\n^\ninvalid number prefix\nexpected only 0x (FIXME)";
        assert_eq!(actual_err.to_string(), expected_err);
    }

    #[test]
    fn test_expression_literal() {
        let mut input = stream_from("0x123;", "<input>");
        let output = parse_expression(&mut input).expect("parse failed");
        assert_eq!(output.span, FileSpan::new("<input>", 0..5));
        assert_eq!(*output, Expression::Literal(0x123));
        assert_eq!(**input, ";");
        // TODO: Test other expression types
        // TODO: Test negative
    }

    #[test]
    fn test_statement_yield() {
        let mut input = stream_from("yield 0x123; Foobar", "<input>");
        let output = parse_statement(&mut input).expect("parse failed");
        // The statement should point at the 'yield' keyword, not the expression!
        assert_eq!(output.span, FileSpan::new("<input>", 0..5));
        assert_eq!(**input, " Foobar");
        if let Statement::Yield(le) = &*output {
            assert_eq!(le.span, FileSpan::new("<input>", 6..11));
            assert_eq!(**le, Expression::Literal(0x123));
        } else {
            panic!("Should have been a yield statement?!");
        }
        // TODO: Test other expression types
        // TODO: Test negative
    }

    #[test]
    fn test_bracedbody_yield() {
        let mut input = stream_from("{   yield  0x123;  }Foobar", "<input>");
        let output = parse_braced_body(&mut input).expect("parse failed");
        // The statement should point at the 'yield' keyword, not the expression!
        assert_eq!(output.span, FileSpan::new("<input>", 0..1));
        assert_eq!(**input, "Foobar");
        assert_eq!(output.len(), 1);
        let ls = &output[0];
        assert_eq!(ls.span, FileSpan::new("<input>", 4..9));
        let le = match &ls.value {
            Statement::Yield(le) => le,
            _ => { panic!("Should have been a yield statement?!"); }
        };
        assert_eq!(le.span, FileSpan::new("<input>", 11..16));
        assert_eq!(**le, Expression::Literal(0x123));
        // TODO: Test other statement types
        // TODO: Test negative
        // TODO: Trailing whitespace after each statement!
        // TODO: Test whitespace-only!
    }

    #[test]
    fn test_functionitem_empty() {
        let mut input = stream_from("function henlo(){}After", "<input>");
        let output = parse_function_item(&mut input).expect("parse failed");
        // The statement should point at the 'yield' keyword, not the expression!
        assert_eq!(output.span, FileSpan::new("<input>", 0..8));
        assert_eq!(**input, "After");
        assert_eq!(output.value.name.value, "henlo");
        assert_eq!(output.value.name.span, FileSpan::new("<input>", 9..14));
        assert_eq!(output.value.argument_list, Vec::new());
        assert_eq!(output.value.return_declaration, None);
        assert_eq!(output.value.body.len(), 0);
        // TODO: Test negative
    }

    #[test]
    fn test_functionitem_empty_whitespace() {
        let mut input = stream_from("function //oof \n\t  henlo  (\t\r\n)//\n{ //Erm. \n }  After", "<input>");
        let output = parse_function_item(&mut input).expect("parse failed");
        // The statement should point at the 'yield' keyword, not the expression!
        assert_eq!(output.span, FileSpan::new("<input>", 0..8));
        assert_eq!(**input, "  After");
        assert_eq!(output.value.name.value, "henlo");
        assert_eq!(output.value.name.span, FileSpan::new("<input>", 19..24));
        assert_eq!(output.value.argument_list, Vec::new());
        assert_eq!(output.value.return_declaration, None);
        assert_eq!(output.value.body.len(), 0);
    }

    #[test]
    fn test_functionitem_simple_yield() {
        let mut input = stream_from("function the_answer() { yield 0x2A; } // Yay!", "<input>");
        let output = parse_function_item(&mut input).expect("parse failed");
        // The statement should point at the 'yield' keyword, not the expression!
        assert_eq!(output.span, FileSpan::new("<input>", 0..8));
        assert_eq!(**input, " // Yay!");
        assert_eq!(output.value.name.value, "the_answer");
        assert_eq!(output.value.name.span, FileSpan::new("<input>", 9..19));
        assert_eq!(output.value.argument_list, Vec::new());
        assert_eq!(output.value.return_declaration, None);
        assert_eq!(output.value.body.len(), 1);
        let yield_stmt = &output.value.body[0];
        assert_eq!(yield_stmt.span, FileSpan::new("<input>", 24..29));
        let le = match &yield_stmt.value {
            Statement::Yield(le) => le,
            _ => { panic!("Should have been a yield statement?!"); }
        };
        assert_eq!(le.span, FileSpan::new("<input>", 30..34));
        assert_eq!(**le, Expression::Literal(42));
    }

    // FIXME: yield
}
