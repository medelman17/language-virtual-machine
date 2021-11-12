use crate::assembler::label_parsers::label_declaration;
// use crate::assembler::opcode_parsers::*;
use crate::assembler::operand_parsers::operand;
// use crate::assembler::register_parsers::register;
use crate::assembler::Token;
// use nom::multispace;
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
            l: opt!(label_declaration) >>
            name: directive_declaration >>
            o1: opt!(operand) >>
            o2: opt!(operand) >>
            o3: opt!(operand) >>
            (
                AssemblerInstruction {
                    opcode: None,
                    directive: Some(name),
                    label: l,
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

#[cfg(test)]
mod tests {

    use super::*;
    use nom::types::CompleteStr;

    #[test]
    fn parser_directive() {
        let result = directive_declaration(CompleteStr(".data"));
        assert_eq!(result.is_ok(), true);
        let (_, directive) = result.unwrap();
        assert_eq!(
            directive,
            Token::Directive {
                name: "data".to_string()
            }
        )
    }

    #[test]
    fn string_directive() {
        let result = directive_combined(CompleteStr("test: .asciiz 'Hello'"));
        assert_eq!(result.is_ok(), true);
        let (_, directive) = result.unwrap();
        let correct_instruction = AssemblerInstruction {
            opcode: None,
            label: Some(Token::LabelDeclaration {
                name: "test".to_string(),
            }),
            directive: Some(Token::Directive {
                name: "asciiz".to_string(),
            }),
            operand_one: Some(Token::IrString {
                name: "Hello".to_string(),
            }),
            operand_two: None,
            operand_three: None,
        };
        assert_eq!(directive, correct_instruction);
    }
}
