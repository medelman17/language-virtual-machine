use crate::assembler::register_parsers::register;
use nom::digit;
use nom::types::CompleteStr;

use crate::assembler::Token;

named!(pub operand<CompleteStr, Token>,
    alt!(
        integer_operand | register
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
