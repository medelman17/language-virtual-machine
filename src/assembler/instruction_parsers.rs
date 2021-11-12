// use crate::assembler::directive_parsers::directive;
use crate::assembler::label_parsers::label_declaration;
use crate::assembler::opcode_parsers::*;
use crate::assembler::operand_parsers::operand;
use crate::assembler::register_parsers::register;
use crate::assembler::symbols::SymbolTable;
use crate::assembler::Token;
use crate::instruction;
use byteorder::{LittleEndian, WriteBytesExt};
use nom::multispace;
use nom::types::CompleteStr;
use nom::*;

use std::fmt;

const MAX_I16: i32 = 32768;
const MIN_I16: i32 = -32768;

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
    pub fn to_bytes(&self, symbols: &SymbolTable) -> Vec<u8> {
        let mut results = vec![];
        // if let Some(ref token) = self.opcode {
        //     match token {
        //         Token::Op { code } => match code {
        //             _ => {
        //                 let b: u8 = (*code).into();
        //                 results.push(b);
        //             }
        //         },
        //     }
        // }
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
                Some(t) => AssemblerInstruction::extract_operand(t, &mut results, symbols),
                None => {}
            }
        }

        while results.len() < 4 {
            results.push(0);
        }

        return results;
    }

    pub fn is_label(&self) -> bool {
        self.label.is_some()
    }

    pub fn is_opcode(&self) -> bool {
        self.opcode.is_some()
    }

    pub fn is_directive(&self) -> bool {
        self.directive.is_some()
    }

    pub fn has_operands(&self) -> bool {
        self.operand_one.is_some() || self.operand_two.is_some() || self.operand_three.is_some()
    }

    pub fn is_integer_needs_splitting(&self) -> bool {
        if let Some(ref op) = self.opcode {
            match op {
                Token::Op { code } => match code {
                    instruction::Opcode::LOAD => {
                        if let Some(ref first_half) = self.operand_two {
                            match first_half {
                                Token::IntegerOperand { ref value } => {
                                    if *value > MAX_I16 || *value < MIN_I16 {
                                        return true;
                                    }
                                    return false;
                                }
                                _ => {
                                    return false;
                                }
                            }
                        }
                        return true;
                    }
                    _ => {
                        return false;
                    }
                },
                _ => {
                    return false;
                }
            }
        }
        false
    }

    pub fn get_integer_value(&self) -> Option<i16> {
        if let Some(ref operand) = self.operand_two {
            match operand {
                Token::IntegerOperand { ref value } => return Some(*value as i16),
                _ => return None,
            }
        }
        None
    }

    pub fn get_register_number(&self) -> Option<u8> {
        match self.operand_one {
            Some(ref reg_token) => match reg_token {
                Token::Register { ref reg_num } => Some(reg_num.clone()),
                _ => None,
            },
            None => None,
        }
    }

    pub fn set_operand_two(&mut self, t: Token) {
        self.operand_two = Some(t)
    }

    pub fn set_operand_three(&mut self, t: Token) {
        self.operand_three = Some(t)
    }

    pub fn get_directive_name(&self) -> Option<String> {
        match &self.directive {
            Some(d) => match d {
                Token::Directive { name } => Some(name.to_string()),
                _ => None,
            },
            None => None,
        }
    }

    pub fn get_string_constant(&self) -> Option<String> {
        match &self.operand_one {
            Some(d) => match d {
                Token::IrString { name } => Some(name.to_string()),
                _ => None,
            },
            None => None,
        }
    }

    pub fn get_i32_constant(&self) -> Option<i32> {
        match &self.operand_one {
            Some(d) => match d {
                Token::IntegerOperand { value } => Some(*value),
                _ => None,
            },
            None => None,
        }
    }

    pub fn get_label_name(&self) -> Option<String> {
        match &self.label {
            Some(l) => match l {
                Token::LabelDeclaration { name } => Some(name.clone()),
                _ => None,
            },
            _ => None,
        }
    }

    fn extract_operand(t: &Token, results: &mut Vec<u8>, symbols: &SymbolTable) {
        match t {
            Token::Register { reg_num } => results.push(*reg_num),
            Token::IntegerOperand { value } => {
                let converted = *value as u16;
                let byte_one = converted;
                let byte_two = converted >> 8;
                results.push(byte_two as u8);
                results.push(byte_one as u8);
            }
            Token::LabelUsage { name } => {
                if let Some(value) = symbols.symbol_value(name) {
                    let mut wtr = vec![];
                    wtr.write_u32::<LittleEndian>(value).unwrap();
                    results.push(wtr[1]);
                    results.push(wtr[0]);
                } else {
                    println!("No value found for {:?}", name);
                    std::process::exit(1);
                }
            }
            _ => {
                println!("Opcode found in operand field");
                std::process::exit(1);
            }
        }
    }
}

impl fmt::Display for AssemblerInstruction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "(Label: {:?} Opcode: {:?} Directive: {:?} Operand #1: {:?} Operand #2: {:?} Operand #3: {:?})",
            self.label,
            self.opcode,
            self.directive,
            self.operand_one,
            self.operand_two,
            self.operand_three
        )
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
