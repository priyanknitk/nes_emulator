use std::collections::HashMap;

use crate::{cpu_flags::CpuFlags, opcodes};

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum AddressingMode {
    Immediate,
    ZeroPage,
    ZeroPage_X,
    ZeroPage_Y,
    Absolute,
    Absolute_X,
    Absolute_Y,
    Indirect_X,
    Indirect_Y,
    NoneAddressing,
}

pub trait Mem {
    fn mem_read(&self, addr: u16) -> u8; 

    fn mem_write(&mut self, addr: u16, data: u8);
    
    fn mem_read_u16(&self, pos: u16) -> u16 {
        let lo = self.mem_read(pos) as u16;
        let hi = self.mem_read(pos + 1) as u16;
        (hi << 8) | (lo as u16)
    }

    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.mem_write(pos, lo);
        self.mem_write(pos + 1, hi);
    }
}

pub struct CPU {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: CpuFlags,
    pub program_counter: u16,
    memory: [u8; 0xffff],
}

impl Mem for CPU {
    fn mem_read(&self, address: u16) -> u8 {
        self.memory[address as usize]
    }

    fn mem_write(&mut self, address: u16, value: u8) {
        self.memory[address as usize] = value;
    }
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status: CpuFlags::from_bits_truncate(0b100100),
            program_counter: 0,
            memory: [0; 0xffff],
        }
    }

    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run();
    }

    pub fn load(&mut self, program: Vec<u8>) {
        self.memory[0x8000..0x8000 + program.len()].copy_from_slice(&program);
        self.mem_write_u16(0xFFFC, 0x8000);
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.status = CpuFlags::from_bits_truncate(0b100100);
        self.program_counter = self.mem_read_u16(0xFFFC);
    }

    pub fn run(&mut self) {
        let ref opcodes: HashMap<u8, &'static opcodes::OpCode> = *opcodes::OPCODES_MAP;
        loop {      
            let code  = self.mem_read(self.program_counter);
            self.program_counter += 1;
            let opcode = opcodes.get(&code).expect("opcode not found");
            let program_counter_state = self.program_counter;
            match code {
                0x00 => return,
                /* LDA  */
                0xa9 | 0xa5 | 0xb5 | 0xad | 0xbd | 0xb9 | 0xa1 | 0xb1 => self.lda(&opcode.mode),
                /* STA */
                0x85 | 0x95 | 0x8d | 0x9d | 0x99 | 0x81 | 0x91 => self.sta(&opcode.mode),
                /* ASL */
                0x0A | 0x06 | 0x16 | 0x0E | 0x1E => self.asl(&opcode.mode),
                /* BCC */
                0x90 => self.bcc(&opcode.mode),
                /* BCS */
                0xB0 => self.bcs(&opcode.mode),
                /* BEQ */
                0xF0 => self.beq(&opcode.mode),
                /* BNE */
                0xD0 => self.bne(&opcode.mode),
                /* BIT */
                0x24 | 0x2C => self.bit(&opcode.mode),
                /* BMI */
                0x30 => self.bmi(&opcode.mode),
                /* BPL */
                0x10 => self.bpl(&opcode.mode),
                /* BVC */
                0x50 => self.bvc(&opcode.mode),
                /* BVS */
                0x70 => self.bvs(&opcode.mode),
                /* CLC */
                0x18 => self.status.remove(CpuFlags::CARRY),
                /* CLD */
                0xD8 => self.status.remove(CpuFlags::DECIMAL_MODE),
                /* CLI */
                0x58 => self.status.remove(CpuFlags::INTERRUPT_DISABLE),
                /* CLV */
                0xB8 => self.status.remove(CpuFlags::OVERFLOW),
                /* TAX */
                0xAA => self.tax(),
                /* INX */
                0xE8 => self.inx(),
                /* AND */
                0x29 | 0x25 | 0x35 | 0x2d | 0x3d | 0x39 | 0x21 | 0x31 => self.and(&opcode.mode),
                _ => todo!(),
            }
            if program_counter_state == self.program_counter {
                self.program_counter += (opcode.len - 1) as u16;
            }
        }
    }

    fn lda(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.set_register_a(value);
    }

    fn tax(&mut self) {
        self.set_register_x(self.register_a);
    }

    fn bcc(&mut self, mode: &AddressingMode) {
        if !self.status.contains(CpuFlags::CARRY) {
            let addr = self.get_operand_address(mode);
            self.program_counter = addr;
        }
    }

    fn bcs(&mut self, mode: &AddressingMode) {
        if self.status.contains(CpuFlags::CARRY) {
            let addr = self.get_operand_address(mode);
            self.program_counter = addr;
        }
    }

    fn beq(&mut self, mode: &AddressingMode) {
        if self.status.contains(CpuFlags::ZERO) {
            let addr = self.get_operand_address(mode);
            self.program_counter = addr;
        }
    }

    fn bne(&mut self, mode: &AddressingMode) {
        if !self.status.contains(CpuFlags::ZERO) {
            let addr = self.get_operand_address(mode);
            self.program_counter = addr;
        }
    }

    fn bit(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        let and = self.register_a & data;
        if and == 0 {
            self.status.insert(CpuFlags::ZERO);
        } else {
            self.status.remove(CpuFlags::ZERO);
        }

        self.status.set(CpuFlags::NEGATIV, data & 0b10000000 > 0);
        self.status.set(CpuFlags::OVERFLOW, data & 0b01000000 > 0);
    }

    fn bmi(&mut self, mode: &AddressingMode) {
        if self.status.contains(CpuFlags::NEGATIV) {
            let addr = self.get_operand_address(mode);
            self.program_counter = addr;
        }
    }

    fn bpl(&mut self, mode: &AddressingMode) {
        if !self.status.contains(CpuFlags::NEGATIV) {
            let addr = self.get_operand_address(mode);
            self.program_counter = addr;
        }
    }

    fn bvc(&mut self, mode: &AddressingMode) {
        if !self.status.contains(CpuFlags::OVERFLOW) {
            let addr = self.get_operand_address(mode);
            self.program_counter = addr;
        }
    }

    fn bvs(&mut self, mode: &AddressingMode) {
        if self.status.contains(CpuFlags::OVERFLOW) {
            let addr = self.get_operand_address(mode);
            self.program_counter = addr;
        }
    }

    fn asl(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        let result = value << 1;
        self.mem_write(addr, result);
        self.update_zero_and_negative_flags(result);
    }

    fn inx(&mut self) {
        self.set_register_x(self.register_x.wrapping_add(1));
    }

    fn and(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.set_register_a(self.register_a & value);
    }

    fn sta(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register_a);
    }

    fn update_zero_and_negative_flags(&mut self, result: u8) {
        if result == 0 {
            self.status.insert(CpuFlags::ZERO);
        } else {
            self.status.remove(CpuFlags::ZERO)
        }

        if result & 0b1000_0000 != 0 {
            self.status.insert(CpuFlags::NEGATIV);
        } else {
            self.status.remove(CpuFlags::NEGATIV);
        }
    }

    fn set_register_a(&mut self, value: u8) {
        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
    }

    fn set_register_x(&mut self, value: u8) {
        self.register_x = value;
        self.update_zero_and_negative_flags(self.register_x);
    }

    fn get_operand_address(&self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::Immediate => self.program_counter,
            AddressingMode::ZeroPage => self.mem_read(self.program_counter) as u16,
            AddressingMode::Absolute => self.mem_read_u16(self.program_counter),
            AddressingMode::ZeroPage_X => {
                let pos = self.mem_read(self.program_counter);
                let addr = pos.wrapping_add(self.register_x) as u16;
                addr
            }
            AddressingMode::ZeroPage_Y => {
                let pos = self.mem_read(self.program_counter);
                let addr = pos.wrapping_add(self.register_y) as u16;
                addr
            }
            AddressingMode::Absolute_X => {
                let base = self.mem_read_u16(self.program_counter);
                let addr = base.wrapping_add(self.register_x as u16);
                addr
            }
            AddressingMode::Absolute_Y => {
                let base = self.mem_read_u16(self.program_counter);
                let addr = base.wrapping_add(self.register_y as u16);
                addr
            }
            AddressingMode::Indirect_X => {
                let base = self.mem_read(self.program_counter);

                let ptr: u8 = (base as u8).wrapping_add(self.register_x);
                let lo = self.mem_read(ptr as u16);
                let hi = self.mem_read(ptr.wrapping_add(1) as u16);
                (hi as u16) << 8 | (lo as u16)
            }
            AddressingMode::Indirect_Y => {
                let base = self.mem_read(self.program_counter);

                let lo = self.mem_read(base as u16);
                let hi = self.mem_read((base as u8).wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                let deref = deref_base.wrapping_add(self.register_y as u16);
                deref
            }
            AddressingMode::NoneAddressing => {
                panic!("mode {:?} is not supported", mode);
            }
        }
    }
}
