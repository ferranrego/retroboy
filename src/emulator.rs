use crate::apu;
use crate::apu::{initialize_apu, ApuState};
use crate::cpu::{self, at_end_of_boot_rom, initialize_cpu, interrupts, timers, CpuState};
use crate::cpu::interrupts::InterruptRegisters;
use crate::cpu::timers::TimerRegisters;
use crate::dma;
use crate::dma::{initialize_dma, DmaState};
use crate::gpu::{self, initialize_gpu, GpuState};
use crate::keys::{initialize_keys, KeyState};
use crate::mmu;
use crate::mmu::{Memory, initialize_memory};
use std::cell::RefMut;
use std::io;

pub struct Emulator {
    pub cpu: CpuState,
    pub interrupts: InterruptRegisters,
    pub timers: TimerRegisters,
    pub memory: Memory,
    pub gpu: GpuState,
    pub keys: KeyState,
    pub apu: ApuState,
    pub dma: DmaState,
    pub render: fn(&[u8]),
}

pub fn initialize_emulator(render: fn(&[u8])) -> Emulator {
    Emulator {
        cpu: initialize_cpu(),
        interrupts: InterruptRegisters {
            enabled: 0,
            flags: 0
        },
        timers: TimerRegisters {
            m_cycles_clock: 0,
            base_clock: 0,
            divider_clock: 0,
            divider: 0,
            counter: 0,
            modulo: 0,
            control: 0
        },
        memory: initialize_memory(),
        gpu: initialize_gpu(),
        keys: initialize_keys(),
        apu: initialize_apu(),
        dma: initialize_dma(),
        render
    }
}

pub fn initialize_screenless_emulator() -> Emulator {
    initialize_emulator(|_| {})
}

pub fn load_rom(emulator: &mut RefMut<Emulator>, rom: &[u8]) -> io::Result<()> {
    let buffer = rom.to_vec();
    mmu::load_rom_buffer(&mut emulator.memory, buffer);
    let cartridge_type = emulator.memory.cartridge_header.type_code;
    if mmu::cartridge_type_supported(cartridge_type) {
        Ok(())
    }
    else {
        let error_message  = format!("Unsupported cartridge type {cartridge_type}."); 
        Err(io::Error::new(io::ErrorKind::Other, error_message)) 
    }
}

pub fn load_bios(emulator: &mut RefMut<Emulator>, bios: &[u8]) {
    mmu::load_bios_buffer_slice(&mut emulator.memory, bios);
}

pub fn skip_bios(emulator: &mut RefMut<Emulator>) {
    cpu::skip_bios(&mut emulator.cpu);
    gpu::skip_bios(&mut emulator.gpu);
    apu::skip_bios(&mut emulator.apu);
    timers::skip_bios(emulator);
    interrupts::skip_bios(emulator);
}

fn transfer_to_game_rom(memory: &mut Memory) {
    memory.in_bios = false;
}

pub fn sync(emulator: &mut Emulator) {
    timers::step(emulator);
    dma::step(emulator);
    gpu::step(emulator);
    apu::step(emulator);
}

pub fn step(emulator: &mut Emulator) {
    if at_end_of_boot_rom(&mut emulator.cpu) {
        transfer_to_game_rom(&mut emulator.memory);
    }

    cpu::opcodes::step(emulator);
}