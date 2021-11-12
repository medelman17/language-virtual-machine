pub mod assembler_errors;
pub mod directive_parsers;
pub mod instruction_parsers;
pub mod label_parsers;
pub mod opcode_parsers;
pub mod operand_parsers;
pub mod program_parsers;
pub mod register_parsers;
pub mod symbols;

use byteorder::{LittleEndian, WriteBytesExt};
use nom::types::CompleteStr;

use crate::assembler::assembler_errors::AssemblerError;
use crate::assembler::instruction_parsers::AssemblerInstruction;
use crate::assembler::program_parsers::{program, Program};
use crate::assembler::symbols::SymbolTable;
use crate::instruction::Opcode;

/// Magic number that begins every bytecode file prefix. These spell out EPIE in ASCII, if you were wondering.
pub const PIE_HEADER_PREFIX: [u8; 4] = [0x45, 0x50, 0x49, 0x45];

/// Constant that determines how long the header is. There are 60 zeros left after the prefix, for later usage if needed.
pub const PIE_HEADER_LENGTH: usize = 64;

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Op { code: Opcode },
    Register { reg_num: u8 },
    IntegerOperand { value: i32 },
    LabelDeclaration { name: String },
    LabelUsage { name: String },
    Directive { name: String },
    IrString { name: String },
}

#[derive(Debug, Default)]
pub struct Assembler {
    pub phase: AssemblerPhase,
    pub symbols: SymbolTable,
    pub ro: Vec<u8>,
    pub bytecode: Vec<u8>,
    ro_offset: u32,
    sections: Vec<AssemblerSection>,
    current_section: Option<AssemblerSection>,
    current_instruction: u32,
    errors: Vec<AssemblerError>,
    buf: [u8; 4],
}

impl Assembler {
    pub fn new() -> Self {
        Assembler {
            phase: AssemblerPhase::First,
            symbols: SymbolTable::new(),
            current_instruction: 0,
            ro_offset: 0,
            ro: vec![],
            bytecode: vec![],
            sections: vec![],
            errors: vec![],
            current_section: None,
            buf: [0, 0, 0, 0],
        }
    }

    pub fn assemble(&mut self, raw: &str) -> Result<Vec<u8>, Vec<AssemblerError>> {
        match program(CompleteStr(raw)) {
            Ok((_remainder, mut program)) => {
                self.process_first_phase(&mut program);

                if !self.errors.is_empty() {
                    error!(
                        "Errors were found in the first parsing phase: {:?}",
                        self.errors
                    );
                    return Err(self.errors.clone());
                }
                debug!("First parsing phase complete");
                debug!("Phase 1 program: {:#?}", program);

                if self.sections.len() != 2 {
                    println!("Did not find at least two sections.");
                    self.errors.push(AssemblerError::InsufficientSections);
                    // TODO: Can we avoid a clone here?
                    return Err(self.errors.clone());
                }

                let mut body = self.process_second_phase(&program);
                let mut assembled_program = self.write_pie_header();

                assembled_program.append(&mut body);
                debug!("Complete program is: {:#?}", assembled_program);

                Ok(assembled_program)
            }
            Err(e) => {
                println!("There was an error assembling the code: {:?}", e);
                Err(vec![AssemblerError::ParseError {
                    error: e.to_string(),
                }])
            }
        }
    }

    fn write_pie_header(&self) -> Vec<u8> {
        let mut header = vec![];
        for byte in PIE_HEADER_PREFIX.into_iter() {
            header.push(byte.clone());
        }

        while header.len() <= PIE_HEADER_LENGTH {
            header.push(0 as u8);
        }
        header
    }

    fn process_first_phase(&mut self, p: &mut Program) {
        info!("Beginning search for LOAD instructions that need to be split up");
        let mut inserts_to_do = Vec::new();
        for (idx, i) in p.instructions.iter_mut().enumerate() {
            if i.is_integer_needs_splitting() {
                let value = i.get_integer_value();
                let _register = i.get_register_number();
                let mut wtr = vec![];
                let _ = wtr.write_i16::<LittleEndian>(value.unwrap());
                i.operand_two = Some(Token::IntegerOperand {
                    value: wtr[1].into(),
                });
                let new_instruction = AssemblerInstruction {
                    opcode: Some(Token::Op { code: Opcode::LUI }),
                    label: None,
                    directive: None,
                    operand_one: i.operand_one.clone(),
                    operand_two: Some(Token::IntegerOperand {
                        value: wtr[0].into(),
                    }),
                    operand_three: None,
                };
                inserts_to_do.push((idx + 1, new_instruction));
            }
        }

        for insert in inserts_to_do {
            p.instructions.insert(insert.0, insert.1)
        }
        info!("Beginning first parsing phase");

        for i in &p.instructions {
            debug!("Parsing instruction: {}", i);
            if i.is_label() {
                // TODO: Factor this out into another function? Put it in `process_label_declaration` maybe?
                if self.current_section.is_some() {
                    // If we have hit a segment header already (e.g., `.code`) then we are ok
                    debug!(
                        "Parsing label declaration in first phase: {:?} with offset {:?}",
                        i.get_label_name(),
                        self.current_instruction * 4
                    );
                    self.process_label_declaration(&i);
                } else {
                    // If we have *not* hit a segment header yet, then we have a label outside of a segment, which is not allowed
                    error!(
                        "Label found outside of a section in first phase: {:?}",
                        i.get_label_name()
                    );
                    self.errors.push(AssemblerError::NoSegmentDeclarationFound {
                        instruction: self.current_instruction,
                    });
                }

                if i.is_directive() {
                    self.process_directive(i);
                }

                // This is used to keep track of which instruction we hit an error on
                self.current_instruction += 1;
            }
            self.phase = AssemblerPhase::Second;
        }
    }

    fn process_second_phase(&mut self, p: &Program) -> Vec<u8> {
        info!("Beginning second parsing phase");

        self.current_instruction = 0;
        let mut program = vec![];
        for i in &p.instructions {
            if i.is_directive() {
                debug!(
                    "Found a directive in second phase {:?}, bypassing",
                    i.directive
                );
                continue;
            }
            if i.is_opcode() {
                let mut bytes = i.to_bytes(&self.symbols);
                program.append(&mut bytes);
            }
            self.current_instruction += 1
        }
        program
    }

    // fn extract_labels(&mut self, p: &Program) {
    //     let mut c = 0;
    //     for i in &p.instructions {
    //         if i.is_label() {
    //             match i.get_label_name() {
    //                 Some(name) => {
    //                     let symbol = Symbol::new_with_offset(name, SymbolType::Label, c);
    //                     self.symbols.add_symbol(symbol);
    //                 }
    //                 None => {}
    //             }
    //         }
    //         c += 4;
    //     }
    // }

    fn process_label_declaration(&mut self, _i: &AssemblerInstruction) {}

    fn process_directive(&mut self, _i: &AssemblerInstruction) {}

    // fn handle_asciiz(&mut self, i: &AssemblerInstruction) {}

    // fn handle_integer(&mut self, i: &AssemblerInstruction) {}

    // fn process_section_header(&mut self, header_name: &str) {}
}

#[derive(Debug, PartialEq, Clone)]
pub enum AssemblerPhase {
    First,
    Second,
}

impl Default for AssemblerPhase {
    fn default() -> Self {
        AssemblerPhase::First
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum AssemblerSection {
    Data { starting_instruction: Option<u32> },
    Code { starting_instruction: Option<u32> },
    Unknown,
}

impl Default for AssemblerSection {
    fn default() -> Self {
        AssemblerSection::Unknown
    }
}

impl<'a> From<&'a str> for AssemblerSection {
    fn from(name: &str) -> AssemblerSection {
        match name {
            "data" => AssemblerSection::Data {
                starting_instruction: None,
            },
            "code" => AssemblerSection::Code {
                starting_instruction: None,
            },
            _ => AssemblerSection::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assembler::symbols::{Symbol, SymbolTable, SymbolType};
    use crate::vm::VirtualMachine;

    #[test]
    fn assemble_program() {
        let mut asm = Assembler::new();
        let test_string = r"
        .data
        .code
        load $0 #100
        load $1 #1
        load $2 #0
        test: inc $0
        neq $0 $2
        jmpe @test
        hlt
        ";
        let program = asm.assemble(test_string).unwrap();
        let mut vm = VirtualMachine::new();
        assert_eq!(program.len(), 96);
        vm.add_bytes(program);
        assert_eq!(vm.program.len(), 96);
    }

    #[test]
    /// Tests that we can add things to the symbol table
    fn test_symbol_table() {
        let mut sym = SymbolTable::new();
        let new_symbol = Symbol::new_with_offset("test".to_string(), SymbolType::Label, 12);
        sym.add_symbol(new_symbol);
        assert_eq!(sym.symbols.len(), 1);
        let v = sym.symbol_value("test");
        assert_eq!(true, v.is_some());
        let v = v.unwrap();
        assert_eq!(v, 12);
        let v = sym.symbol_value("does_not_exist");
        assert_eq!(v.is_some(), false);
    }

    #[test]
    /// Simple test of data that goes into the read only section
    fn test_ro_data_asciiz() {
        let mut asm = Assembler::new();
        let test_string = r"
        .data
        test: .asciiz 'This is a test'
        .code
        ";
        let program = asm.assemble(test_string);
        assert_eq!(program.is_ok(), true);
    }

    #[test]
    /// Simple test of data that goes into the read only section
    fn test_code_start_offset_written() {
        let mut asm = Assembler::new();
        let test_string = r"
        .data
        test1: .asciiz 'Hello'
        .code
        load $0 #100
        load $1 #1
        load $2 #0
        test: inc $0
        neq $0 $2
        jmpe @test
        hlt
        ";
        let program = asm.assemble(test_string);
        assert_eq!(program.is_ok(), true);
        let unwrapped = program.unwrap();
        assert_eq!(unwrapped[64], 6);
    }

    #[test]
    /// Simple test of data that goes into the read only section
    fn test_ro_data_i32() {
        let mut asm = Assembler::new();
        let test_string = r"
        .data
        test: .integer #300
        .code
        ";
        let program = asm.assemble(test_string);
        assert_eq!(program.is_ok(), true);
    }

    #[test]
    /// This tests that a section name that isn't `code` or `data` throws an error
    fn test_bad_ro_data() {
        let mut asm = Assembler::new();
        let test_string = r"
        .code
        test: .asciiz 'This is a test'
        .wrong
        ";
        let program = asm.assemble(test_string);
        assert_eq!(program.is_ok(), false);
    }

    #[test]
    /// Tests that code which does not declare a segment first does not work
    fn test_first_phase_no_segment() {
        let mut asm = Assembler::new();
        let test_string = "hello: .asciiz 'Fail'";
        let result = program(CompleteStr(test_string));
        assert_eq!(result.is_ok(), true);
        let (_, mut p) = result.unwrap();
        asm.process_first_phase(&mut p);
        assert_eq!(asm.errors.len(), 1);
    }

    #[test]
    /// Tests that code inside a proper segment works
    fn test_first_phase_inside_segment() {
        let mut asm = Assembler::new();
        let test_string = r"
        .data
        test: .asciiz 'Hello'
        ";
        let result = program(CompleteStr(test_string));
        assert_eq!(result.is_ok(), true);
        let (_, mut p) = result.unwrap();
        asm.process_first_phase(&mut p);
        assert_eq!(asm.errors.len(), 0);
    }
}

// #[test]
// fn assemble_program() {
//     let mut asm = Assembler::new();
//     let test_string =
//         "load $0 #100\nload $1 #1\nload $2 #0\ntest: inc $0\nneq $0 $2\njmpe @test\nhlt";
//     let program = asm.assemble(test_string).unwrap();
//     let mut vm = VirtualMachine::new();
//     assert_eq!(program.len(), 81);
//     vm.add_bytes(program);
//     assert_eq!(vm.program.len(), 81);
// }
