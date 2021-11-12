use crate::assembler::label_parsers::label_usage;
use crate::assembler::register_parsers::register;
use nom::digit;
use nom::types::CompleteStr;

use crate::assembler::Token;

named!(pub operand<CompleteStr, Token>,
    alt!(
        integer_operand | register | irstring | label_usage
    )
);

named!(irstring<CompleteStr, Token>,
    do_parse!(
        tag!("'") >>
        content: take_until!("'") >>
        tag!("'") >>
        (
            Token::IrString{ name: content.to_string()}
        )
    )
);

named!( integer_operand<CompleteStr, Token>,
    ws!(
        do_parse!(
            tag!("#") >>
            reg_num: digit >>
            (
                Token::IntegerOperand{value: reg_num.parse::<i32>().unwrap()}
            )
        )
    )
);

#[test]
fn parse_integer_operand() {
    let result = integer_operand(CompleteStr("#10"));
    assert_eq!(result.is_ok(), true);
    let (rest, value) = result.unwrap();
    assert_eq!(rest, CompleteStr(""));
    assert_eq!(value, Token::IntegerOperand { value: 10 });

    // Test an invalid one (missing the #)
    let result = integer_operand(CompleteStr("10"));
    assert_eq!(result.is_ok(), false);
}

#[test]
fn parse_string_operand() {
    let result = irstring(CompleteStr("'This is a test'"));
    assert_eq!(result.is_ok(), true);
}
