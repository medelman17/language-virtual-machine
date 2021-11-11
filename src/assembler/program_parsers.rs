use crate::assembler::instruction_parsers::{instruction, AssemblerInstruction};
use nom::types::CompleteStr;

#[derive(Debug, PartialEq)]
pub struct Program {
    instructions: Vec<AssemblerInstruction>,
}

impl Program {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut program = vec![];
        for instruction in &self.instructions {
            program.append(&mut instruction.to_bytes());
        }
        program
    }
}

named!(pub program<CompleteStr, Program>,
    do_parse!(
        instructions: many1!(instruction) >>
        (
            Program {
                instructions: instructions
            }
        )
    )
);

#[test]
fn parse_program() {
    let result = program(CompleteStr("load $0 #100\n"));
    assert_eq!(result.is_ok(), true);
    let (leftover, p) = result.unwrap();
    assert_eq!(leftover, CompleteStr(""));
    assert_eq!(1, p.instructions.len());
}

#[test]
fn test_program_to_bytes() {
    let result = program(CompleteStr("load $0 #100\n"));
    assert_eq!(result.is_ok(), true);
    let (_, program) = result.unwrap();
    let bytecode = program.to_bytes();
    assert_eq!(bytecode.len(), 4);
    println!("{:?}", bytecode);
}
