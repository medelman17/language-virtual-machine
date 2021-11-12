use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

#[macro_use]
extern crate nom;

#[macro_use]
extern crate clap;

#[macro_use]
extern crate log;

extern crate serde;
extern crate serde_derive;

use clap::App;

use crate::vm::VirtualMachine;

pub mod assembler;
pub mod instruction;
pub mod repl;
pub mod utils;
pub mod vm;

extern crate env_logger;

fn main() {
    env_logger::init();
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();
    let target_file = matches.value_of("INPUT_FILE");
    match target_file {
        Some(filename) => {
            let program = read_file(filename);
            let mut asm = assembler::Assembler::new();
            let mut vm = VirtualMachine::new();
            let program = asm.assemble(&program);
            match program {
                Ok(p) => {
                    vm.add_bytes(p);
                    vm.run();
                    println!("{:#?}", vm.registers);
                    std::process::exit(0)
                }
                Err(_e) => {}
            }
        }
        None => {
            start_repl();
        }
    }
}

fn start_repl() {
    let mut repl = repl::REPL::new();
    repl.run();
}

fn read_file(tmp: &str) -> String {
    let filename = Path::new(tmp);
    match File::open(Path::new(&filename)) {
        Ok(mut fh) => {
            let mut contents = String::new();
            match fh.read_to_string(&mut contents) {
                Ok(_) => {
                    return contents;
                }
                Err(e) => {
                    println!("There was an error reading file: {:?}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            println!("File not found: {:?}", e);
            std::process::exit(1)
        }
    }
}
