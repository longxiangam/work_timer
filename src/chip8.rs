use chip8::cpu::Cpu;
use chip8::display::Display;
use chip8::keypad::Keypad;
use chip8::rand::{ComplementaryMultiplyWithCarryGen, CMWC_CYCLE};


pub struct Chip8{
    cpu:Cpu
}

impl Chip8{
    pub fn new()->Chip8{
        let cpu = Cpu {
            i: 0,
            pc: 0,
            dt: 0,
            memory: [0; 4096],
            v: [0; 16],
            display: Display {
                memory: [0; 2048]
            },
            keypad: Keypad {
                keys: [false; 16]
            },
            stack: [0; 16],
            sp: 0,
            rand: ComplementaryMultiplyWithCarryGen {
                q: [0; CMWC_CYCLE],
                c: 0,
                i: 0
            }
        };

        Chip8 { cpu }
    }

    pub fn load_rom(&mut self,rom:& [u8]){
        let offset = 0x200;
        for (index,byte) in rom.iter().enumerate() {
            self.cpu.memory[offset+index] = *byte;
        }
    }

    pub fn run(&mut self){
        for  i in 0..30 {
            self.execute_cycle();
        }
        self.decrement_timers();
    }



    pub fn reset(&mut self) {
            self.cpu.reset();
    }

    pub fn get_memory(& mut self) -> & [u8; 4096] {
        &self.cpu.memory
    }

    pub fn get_display(&mut self) -> & [u8; 2048] {
        &self.cpu.display.memory
    }

    pub fn key_down(&mut self,i: u8) {
        self.cpu.keypad.key_down(i);
    }

    pub fn key_up(&mut self,i: u8) {
        self.cpu.keypad.key_up(i);
    }

    pub fn get_register_v(&mut self) -> & [u8; 16] {
        &self.cpu.v
    }

    pub fn get_register_i(&mut self) -> u16 {
        self.cpu.i
    }


    pub fn get_register_pc(&mut self) -> u16 {
        self.cpu.pc
    }


    pub fn execute_cycle(&mut self) {
        self.cpu.execute_cycle();
    }

    pub fn decrement_timers(&mut self) {
        self.cpu.decrement_timers();
    }
}

