use std;
use std::io::Cursor;
// use std::net::SocketAddr;
// use std::sync::{Arc, RwLock};
// use std::thread;

use byteorder::*;
use chrono::prelude::*;
use num_cpus;
use uuid::Uuid;

use crate::assembler::{PIE_HEADER_LENGTH, PIE_HEADER_PREFIX};
use crate::instruction::Opcode;

/// Default starting size for a VM's heap
pub const DEFAULT_HEAP_STARTING_SIZE: usize = 64;

/// Default stack starting space. We'll default to 2MB.
pub const DEFAULT_STACK_SPACE: usize = 2097152;

#[derive(Clone, Debug)]
pub enum VMEventType {
    Start,
    GracefulStop { code: u32 },
    Crash { code: u32 },
}

impl VMEventType {
    pub fn stop_code(&self) -> u32 {
        match &self {
            VMEventType::Start => 0,
            VMEventType::GracefulStop { code } => *code,
            VMEventType::Crash { code } => *code,
        }
    }
}

#[derive(Clone, Debug)]
/// Struct for a VMEvent that includes the application ID and time
pub struct VMEvent {
    pub event: VMEventType,
    at: DateTime<Utc>,
    application_id: Uuid,
}

pub struct VirtualMachine {
    /// Array that simulates having hardware registers
    pub registers: [i32; 32],
    /// Array that simulates having floating point hardware registers
    pub float_registers: [f64; 32],
    pub logical_cores: usize,
    pub stack: Vec<i32>,
    pub loop_counter: usize,
    pub id: Uuid,
    events: Vec<VMEvent>,

    /// Program counter that tracks which byte is being executed
    pc: usize,
    /// Keeps track of where in the stack the program currently is
    pub sp: usize,
    /// Keeps track of the current frame pointer
    pub bp: usize,

    /// Bytecode of the program being run
    pub program: Vec<u8>,
    /// Remainder of modulo division ops
    remainder: u32,
    /// Result of last comparison op
    equal_flag: bool,
    heap: Vec<u8>,
    /// Contains the read-only section data
    ro_data: Vec<u8>,
    alias: Option<String>,
}

impl VirtualMachine {
    pub fn new() -> Self {
        VirtualMachine {
            id: Uuid::new_v4(),
            events: Vec::new(),
            logical_cores: num_cpus::get(),
            loop_counter: 0,
            stack: Vec::with_capacity(DEFAULT_STACK_SPACE),
            registers: [0; 32],
            float_registers: [0.0; 32],
            program: vec![],
            pc: 0,
            sp: 0,
            bp: 0,
            remainder: 0,
            equal_flag: false,
            heap: vec![0, DEFAULT_HEAP_STARTING_SIZE as u8],
            ro_data: vec![],
            alias: None,
        }
    }

    pub fn with_alias(mut self, alias: String) -> Self {
        if alias == "" {
            self.alias = None;
        } else {
            self.alias = Some(alias);
        }
        self
    }

    /// Loops as long as instructions can be executed.
    pub fn run(&mut self) -> Vec<VMEvent> {
        self.events.push(VMEvent {
            event: VMEventType::Start,
            at: Utc::now(),
            application_id: self.id,
        });

        if !self.verify_header() {
            self.events.push(VMEvent {
                event: VMEventType::Crash { code: 1 },
                at: Utc::now(),
                application_id: self.id,
            });
            error!("Header was incorrect");
            return self.events.clone();
        }

        self.pc = 68 + self.get_starting_offset();
        let mut is_done = None;
        while is_done.is_none() {
            is_done = self.execute_instruction();
        }
        self.events.push(VMEvent {
            event: VMEventType::GracefulStop {
                code: is_done.unwrap(),
            },
            at: Utc::now(),
            application_id: self.id,
        });
        self.events.clone()
    }

    /// Executes one instruction. Meant to allow for more controlled execution.
    pub fn run_once(&mut self) {
        self.execute_instruction();
    }

    pub fn add_byte(&mut self, b: u8) {
        self.program.push(b);
    }

    pub fn add_bytes(&mut self, mut b: Vec<u8>) {
        self.program.append(&mut b);
    }

    pub fn get_test_vm() -> Self {
        let mut vm = VirtualMachine::new();
        vm.registers[0] = 5;
        vm.registers[1] = 10;
        vm
    }

    fn verify_header(&self) -> bool {
        if self.program[0..4] != PIE_HEADER_PREFIX {
            return false;
        }
        true
    }

    fn execute_instruction(&mut self) -> Option<u32> {
        if self.pc >= self.program.len() {
            return Some(1);
        }

        match self.decode_opcode() {
            Opcode::ADD => {
                let register_one = self.registers[self.next_eight_bits() as usize];
                let register_two = self.registers[self.next_eight_bits() as usize];
                self.registers[self.next_eight_bits() as usize] = register_one + register_two;
            }
            Opcode::SUB => {
                let register_one = self.registers[self.next_eight_bits() as usize];
                let register_two = self.registers[self.next_eight_bits() as usize];
                self.registers[self.next_eight_bits() as usize] = register_one - register_two;
            }
            Opcode::MUL => {
                let register_one = self.registers[self.next_eight_bits() as usize];
                let register_two = self.registers[self.next_eight_bits() as usize];
                self.registers[self.next_eight_bits() as usize] = register_one * register_two;
            }
            Opcode::DIV => {
                let register_one = self.registers[self.next_eight_bits() as usize];
                let register_two = self.registers[self.next_eight_bits() as usize];
                self.registers[self.next_eight_bits() as usize] = register_one / register_two;
                self.remainder = (register_one % register_two) as u32;
            }
            Opcode::LOAD => {
                let register = self.next_eight_bits() as usize;
                let number = self.next_sixteen_bits() as u16;
                self.registers[register] = number as i32;
            }
            Opcode::HLT => {
                println!("HLT encountered");
                return Some(1);
            }
            Opcode::JMP => {
                let target = self.registers[self.next_eight_bits() as usize];
                self.pc = target as usize;
            }
            Opcode::JMPB => {
                let value = self.registers[self.next_eight_bits() as usize];
                self.pc -= value as usize;
            }
            Opcode::JMPF => {
                let value = self.registers[self.next_eight_bits() as usize];
                self.pc += value as usize;
            }
            Opcode::EQ => {
                let register_one = self.registers[self.next_eight_bits() as usize];
                let register_two = self.registers[self.next_eight_bits() as usize];
                if register_one == register_two {
                    self.equal_flag = true;
                } else {
                    self.equal_flag = false;
                }
                self.next_eight_bits();
            }

            Opcode::NEQ => {
                let register_one = self.registers[self.next_eight_bits() as usize];
                let register_two = self.registers[self.next_eight_bits() as usize];
                self.equal_flag = register_one != register_two;
                self.next_eight_bits();
            }
            Opcode::GT => {
                let register_one = self.registers[self.next_eight_bits() as usize];
                let register_two = self.registers[self.next_eight_bits() as usize];
                self.equal_flag = register_one > register_two;
                self.next_eight_bits();
            }
            Opcode::LT => {
                let register_one = self.registers[self.next_eight_bits() as usize];
                let register_two = self.registers[self.next_eight_bits() as usize];
                self.equal_flag = register_one < register_two;
                self.next_eight_bits();
            }
            Opcode::GTQ => {
                let register_one = self.registers[self.next_eight_bits() as usize];
                let register_two = self.registers[self.next_eight_bits() as usize];
                self.equal_flag = register_one >= register_two;
                self.next_eight_bits();
            }
            Opcode::LTQ => {
                let register_one = self.registers[self.next_eight_bits() as usize];
                let register_two = self.registers[self.next_eight_bits() as usize];
                self.equal_flag = register_one <= register_two;
                self.next_eight_bits();
            }
            Opcode::JEQ => {
                let register = self.next_eight_bits() as usize;
                let target = self.registers[register];
                if self.equal_flag {
                    self.pc = target as usize;
                }
            }
            Opcode::JNEQ => {
                let register = self.next_eight_bits() as usize;
                let target = self.registers[register];
                if !self.equal_flag {
                    self.pc = target as usize;
                }
            }
            Opcode::ALOC => {
                let register = self.next_eight_bits() as usize;
                let bytes = self.registers[register];
                let new_end = self.heap.len() as i32 + bytes;
                self.heap.resize(new_end as usize, 0);
            }
            Opcode::IGL => {
                println!("Illegal instruction encountered");
                // This was false
                return Some(1);
            }
            Opcode::INC => {
                let register = self.next_eight_bits() as usize;
                self.registers[register] += 1;
                self.next_eight_bits();
                self.next_eight_bits();
            }
            Opcode::DEC => {
                let register = self.next_eight_bits() as usize;
                self.registers[register] -= 1;
                self.next_eight_bits();
                self.next_eight_bits();
            }
            Opcode::LUI => {}
            Opcode::PRTS => {
                let starting_offset = self.next_sixteen_bits() as usize;
                let mut ending_offset = starting_offset;
                let slice = self.ro_data.as_slice();
                while slice[ending_offset] != 0 {
                    ending_offset += 1;
                }
                let result = std::str::from_utf8(&slice[starting_offset..ending_offset]);
                match result {
                    Ok(s) => {
                        print!("{}", s);
                    }
                    Err(e) => {
                        println!("Error decoding string for prts instruction: {:#?}", e)
                    }
                };
            }
        }
        None
    }

    pub fn print_i32_register(&self, register: usize) {
        let bits = self.registers[register];
        println!("bits: {:#032b}", bits);
    }

    fn decode_opcode(&mut self) -> Opcode {
        let opcode = Opcode::from(self.program[self.pc]);
        self.pc += 1;
        return opcode;
    }

    fn get_starting_offset(&self) -> usize {
        let mut rdr = Cursor::new(&self.program[64..68]);
        rdr.read_i32::<LittleEndian>().unwrap() as usize
    }

    fn _i32_to_bytes(num: i32) -> [u8; 4] {
        let mut buf: [u8; 4] = [0, 0, 0, 0];
        buf.as_mut().write_i32::<LittleEndian>(num).unwrap();
        buf
    }

    fn next_eight_bits(&mut self) -> u8 {
        let result = self.program[self.pc];
        self.pc += 1;
        return result;
    }

    fn next_sixteen_bits(&mut self) -> u16 {
        let result = ((self.program[self.pc] as u16) << 8) | self.program[self.pc + 1] as u16;
        self.pc += 2;
        return result;
    }

    pub fn prepend_header(mut b: Vec<u8>) -> Vec<u8> {
        let mut prepension = vec![];
        for byte in PIE_HEADER_PREFIX.into_iter() {
            prepension.push(byte.clone());
        }

        while prepension.len() < PIE_HEADER_LENGTH + 4 {
            prepension.push(0);
        }
        prepension.append(&mut b);
        prepension
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_virtual_machine() {
        let vm = VirtualMachine::new();
        assert_eq!(vm.registers[0], 0);
        assert_eq!(vm.pc, 0);
    }

    #[test]
    fn opcode_hlt() {
        let mut vm = VirtualMachine::new();
        let bytes = vec![5, 0, 0, 0];
        vm.program = bytes;
        vm.run_once();
        assert_eq!(vm.pc, 1);
    }

    #[test]
    fn opcode_igl() {
        let mut vm = VirtualMachine::new();
        let bytes = vec![200, 0, 0, 0];
        vm.program = bytes;
        vm.run_once();
        assert_eq!(vm.pc, 1);
    }

    #[test]
    fn opcode_load() {
        let mut vm = VirtualMachine::new();
        vm.program = vec![0, 0, 1, 244];
        vm.run_once();
        assert_eq!(vm.registers[0], 500);
    }

    #[test]
    fn opcode_jmp() {
        let mut vm = VirtualMachine::new();
        vm.registers[0] = 1;
        vm.program = vec![6, 0, 0, 0];
        vm.run_once();
        assert_eq!(vm.pc, 1);
    }

    #[test]
    fn opcode_jmpf() {
        let mut vm = VirtualMachine::new();
        vm.registers[0] = 2;
        vm.program = vec![7, 0, 0, 0, 5, 0, 0, 0];
        vm.run_once();
        assert_eq!(vm.pc, 4);
    }

    #[test]
    fn opcode_jmpb() {
        let mut vm = VirtualMachine::new();
        vm.registers[0] = 2;
        vm.program = vec![8, 0, 0, 0, 5, 0, 0, 0];
        vm.run_once();
        assert_eq!(vm.pc, 0);
    }

    #[test]
    fn opcode_eq() {
        let mut vm = VirtualMachine::new();
        vm.registers[0] = 10;
        vm.registers[1] = 10;
        vm.program = vec![9, 0, 1, 0, 9, 0, 1, 0];
        vm.run_once();
        assert_eq!(vm.equal_flag, true);
        vm.registers[1] = 20;
        vm.run_once();
        assert_eq!(vm.equal_flag, false);
    }

    #[test]
    fn opcode_neq() {
        let mut vm = VirtualMachine::new();
        vm.registers[0] = 10;
        vm.registers[1] = 20;
        vm.program = vec![10, 0, 1, 0, 10, 0, 1, 0];
        vm.run_once();
        assert_eq!(vm.equal_flag, true);
        vm.registers[1] = 10;
        vm.run_once();
        assert_eq!(vm.equal_flag, false);
    }

    #[test]
    fn opcode_gt() {
        let mut vm = VirtualMachine::new();
        vm.registers[0] = 20;
        vm.registers[1] = 10;
        vm.program = vec![11, 0, 1, 0, 11, 0, 1, 0, 11, 0, 1, 0];
        vm.run_once();
        assert_eq!(vm.equal_flag, true);
        vm.registers[0] = 10;
        vm.run_once();
        assert_eq!(vm.equal_flag, false);
        vm.registers[0] = 5;
        vm.run_once();
        assert_eq!(vm.equal_flag, false);
    }

    #[test]
    fn opcode_jeq() {
        let mut vm = VirtualMachine::new();
        vm.registers[0] = 7;
        vm.equal_flag = true;
        vm.program = vec![15, 0, 0, 0, 17, 0, 0, 0, 17, 0, 0, 0];
        vm.run_once();
        assert_eq!(vm.pc, 7);
    }

    #[test]
    fn opcode_jneq() {
        let mut vm = VirtualMachine::new();
        vm.registers[0] = 7;
        vm.equal_flag = false;
        vm.program = vec![16, 0, 0, 0, 17, 0, 0, 0, 17, 0, 0, 0];
        vm.run_once();
        assert_eq!(vm.pc, 7);
    }

    #[test]
    fn opcode_aloc() {
        let mut vm = VirtualMachine::new();
        vm.registers[0] = 1024;
        vm.program = vec![17, 0, 0, 0];
        vm.run_once();
        assert_eq!(vm.heap.len(), 1024);
    }

    #[test]
    fn opcode_inc() {
        let mut vm = VirtualMachine::new();
        vm.registers[0] = 1;
        vm.program = vec![18, 0, 0, 0];
        vm.run_once();
        assert_eq!(vm.registers[0], 2);
    }

    #[test]
    fn opcode_dec() {
        let mut vm = VirtualMachine::new();
        vm.registers[0] = 1;
        vm.program = vec![19, 0, 0, 0];
        vm.run_once();
        assert_eq!(vm.registers[0], 0);
    }

    #[test]
    fn opcode_mul() {
        let mut vm = VirtualMachine::get_test_vm();
        vm.program = vec![3, 0, 1, 2];
        vm.program = VirtualMachine::prepend_header(vm.program);
        vm.run();
        assert_eq!(vm.registers[2], 50);
    }
}
