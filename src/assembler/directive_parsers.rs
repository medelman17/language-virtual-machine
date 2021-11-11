use crate::assembler::opcode_parsers::*;
use crate::assembler::operand_parsers::operand;
use crate::assembler::register_parsers::register;
use crate::assembler::Token;
use nom::multispace;
use nom::types::CompleteStr;
use nom::*;

use super::instruction_parsers::AssemblerInstruction;

named!(directive_declaration<CompleteStr, Token>,
    do_parse!(
        tag!(".") >>
        name: alpha1 >>
        (
            Token::Directive{name: name.to_string()}
        )
    )
);

named!(directive_combined<CompleteStr, AssemblerInstruction>,
    ws!(
        do_parse!(
            tag!(".") >>
            name: directive_declaration >>
            o1: opt!(operand) >>
            o2: opt!(operand) >>
            o3: opt!(operand) >>
            (
                AssemblerInstruction {
                    opcode: None,
                    directive: Some(name),
                    label: None,
                    operand_one: o1,
                    operand_two: o2,
                    operand_three: o3,
                }
            )
        )
    )
);

named!(pub directive<CompleteStr, AssemblerInstruction>,
    do_parse!(
        ins: alt!(
            directive_combined
        ) >> (
            ins
        )
    )
);
