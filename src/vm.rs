use crate::instruction::Opcode;

pub struct VirtualMachine {
    /// Array that simulates having hardware registers
    pub registers: [i32; 32],
    /// Counter that tracks which byte is being executed
    pc: usize,
    /// Bytecode of the program being run
    pub program: Vec<u8>,
    /// Remainder of modulo division ops
    remainder: u32,
    /// Result of last comparison op
    equal_flag: bool,
    heap: Vec<u8>,
}

impl VirtualMachine {
    pub fn new() -> Self {
        VirtualMachine {
            registers: [0; 32],
            program: vec![],
            pc: 0,
            remainder: 0,
            equal_flag: false,
            heap: vec![],
        }
    }

    /// Loops as long as instructions can be executed.
    pub fn run(&mut self) {
        let mut is_done = false;
        while !is_done {
            is_done = self.execute_instruction();
        }
    }

    /// Executes one instruction. Meant to allow for more controlled execution.
    pub fn run_once(&mut self) {
        self.execute_instruction();
    }

    pub fn add_byte(&mut self, b: u8) {
        self.program.push(b);
    }

    fn execute_instruction(&mut self) -> bool {
        if self.pc >= self.program.len() {
            return true;
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
                return true;
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
                return true;
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
        }
        false
    }

    fn decode_opcode(&mut self) -> Opcode {
        let opcode = Opcode::from(self.program[self.pc]);
        self.pc += 1;
        return opcode;
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
}
