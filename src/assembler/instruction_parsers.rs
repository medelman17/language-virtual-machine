use crate::assembler::directive_parsers::directive;
use crate::assembler::label_parsers::label_declaration;
use crate::assembler::opcode_parsers::*;
use crate::assembler::operand_parsers::operand;
use crate::assembler::register_parsers::register;
use crate::assembler::Token;
use nom::multispace;
use nom::types::CompleteStr;
use nom::*;

#[derive(Debug, PartialEq)]
pub struct AssemblerInstruction {
    pub opcode: Option<Token>,
    pub label: Option<Token>,
    pub directive: Option<Token>,
    pub operand_one: Option<Token>,
    pub operand_two: Option<Token>,
    pub operand_three: Option<Token>,
}
impl AssemblerInstruction {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut results = vec![];
        match self.opcode {
            Some(Token::Op { code }) => match code {
                _ => {
                    results.push(code as u8);
                }
            },
            _ => {
                println!("Non-opcode found in opcode field");
                std::process::exit(1);
            }
        };

        for operand in vec![&self.operand_one, &self.operand_two, &self.operand_three] {
            match operand {
                Some(t) => AssemblerInstruction::extract_operand(t, &mut results),
                None => {}
            }
        }

        while results.len() < 4 {
            results.push(0);
        }

        return results;
    }

    fn extract_operand(t: &Token, results: &mut Vec<u8>) {
        match t {
            Token::Register { reg_num } => results.push(*reg_num),
            Token::IntegerOperand { value } => {
                let converted = *value as u16;
                let byte_one = converted;
                let byte_two = converted >> 8;
                results.push(byte_two as u8);
                results.push(byte_one as u8);
            }
            _ => {
                println!("Opcode found in operand field");
                std::process::exit(1);
            }
        }
    }
}

named!(instruction_combined<CompleteStr, AssemblerInstruction>,
    do_parse!(
        l: opt!(label_declaration) >>
        o: opcode >>
        o1: opt!(operand) >>
        o2: opt!(operand) >>
        o3: opt!(operand) >>
        (
            AssemblerInstruction {
                opcode: Some(o),
                label: l,
                directive: None,
                operand_one: o1,
                operand_two: o2,
                operand_three: o3
            }
        )
    )

);

named!(pub instruction<CompleteStr, AssemblerInstruction>,
    do_parse!(
        ins: alt!(
            instruction_one |
            instruction_two |
            instruction_three
        ) >>
        (
            ins
        )
    )
);

named!(instruction_three <CompleteStr,AssemblerInstruction>,
    do_parse!(
        o: opcode
            >> register_one: register
            >> register_two: register
            >> register_three: register
            >> (AssemblerInstruction {
                label: None,
                directive: None,
                opcode: Some(o),
                operand_one: Some(register_one),
                operand_two: Some(register_two),
                operand_three: Some(register_three)
            })
    )
);

named!(instruction_two<CompleteStr, AssemblerInstruction>,
    do_parse!(
        o: opcode >>
        opt!(multispace) >>
        (
            AssemblerInstruction{
                label: None,
                directive: None,
                opcode: Some(o),
                operand_one: None,
                operand_two: None,
                operand_three: None
            }
        )
    )
);

named!(instruction_one<CompleteStr, AssemblerInstruction>,
    do_parse!(
        o: opcode >>
        r: register >>
        i: operand >>
        (
            AssemblerInstruction{
                label: None,
                directive: None,
                opcode: Some(o),
                operand_one: Some(r),
                operand_two: Some(i),
                operand_three: None
            }
        )
    )
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::Opcode;

    #[test]
    fn parse_instruction_form_one() {
        let result = instruction_one(CompleteStr("load $0 #100\n"));
        assert_eq!(
            result,
            Ok((
                CompleteStr(""),
                AssemblerInstruction {
                    label: None,
                    directive: None,
                    opcode: Some(Token::Op { code: Opcode::LOAD }),
                    operand_one: Some(Token::Register { reg_num: 0 }),
                    operand_two: Some(Token::IntegerOperand { value: 100 }),
                    operand_three: None
                }
            ))
        )
    }

    #[test]
    fn parse_instruction_form_two() {
        let result = instruction_two(CompleteStr("hlt\n"));
        assert_eq!(
            result,
            Ok((
                CompleteStr(""),
                AssemblerInstruction {
                    label: None,
                    directive: None,
                    opcode: Some(Token::Op { code: Opcode::HLT }),
                    operand_one: None,
                    operand_two: None,
                    operand_three: None
                }
            ))
        );
    }

    #[test]
    fn parse_instruction_form_three() {
        let result = instruction_three(CompleteStr("add $0 $1 $2\n"));
        assert_eq!(
            result,
            Ok((
                CompleteStr(""),
                AssemblerInstruction {
                    label: None,
                    directive: None,
                    opcode: Some(Token::Op { code: Opcode::ADD }),
                    operand_one: Some(Token::Register { reg_num: 0 }),
                    operand_two: Some(Token::Register { reg_num: 1 }),
                    operand_three: Some(Token::Register { reg_num: 2 })
                }
            ))
        )
    }
}
