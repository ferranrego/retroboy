use crate::emulator::Emulator;
use crate::gpu::constants::{GB_SCREEN_HEIGHT, GB_SCREEN_WIDTH};
use crate::gpu::scanline::write_scanline;
use crate::gpu::sprites::{collect_scanline_sprites, Sprite};
use crate::utils::is_bit_set;

#[derive(Debug)]
pub struct GpuRegisters {
    pub lcdc: u8,
    pub scy: u8,
    pub scx: u8,
    pub wx: u8,
    pub wy: u8,
    pub palette: u8,
    pub ly: u8,
    pub lyc: u8,
    pub stat: u8,
    pub obp0: u8,
    pub obp1: u8
}

#[derive(Debug)]
pub struct GpuState {
    pub mode: u8,
    pub mode_clock: u16,
    pub registers: GpuRegisters,
    pub frame_buffer: Vec<u32>,
    pub sprite_buffer: Vec<Sprite>
}

const OAM_MODE: u8 = 2;
const OAM_TIME: u16 = 80;

const VRAM_MODE: u8 = 3;
const VRAM_TIME: u16 = 172;

const HBLANK_MODE: u8 = 0;
const HBLANK_TIME: u16 = 204;

const VBLANK_MODE: u8 = 1;

const SCANLINE_RENDER_TIME: u16 = 456;

const FRAME_SCANLINE_COUNT: u8 = 154;
const VBLANK_SCANLINE_COUNT: u8 = 10;

const STAT_INTERRUPT_LYC_CHECK_BIT: u8 = 6;
const OAM_MODE_STAT_SOURCE_BIT: u8 = 5;
const VBLANK_MODE_STAT_SOURCE_BIT: u8 = 4;
const HBLANK_MODE_STAT_SOURCE_BIT: u8 = 3;

pub fn initialize_gpu() -> GpuState {
    GpuState {
        mode: 2,
        mode_clock: 0,
        registers: GpuRegisters {
            lcdc: 0,
            scy: 0,
            scx: 0,
            wx: 0,
            wy: 0,
            palette: 0,
            ly: 0,
            lyc: 0,
            stat: 0,
            obp0: 0,
            obp1: 0
        },
        frame_buffer: vec![0; (GB_SCREEN_WIDTH * GB_SCREEN_HEIGHT) as usize],
        sprite_buffer: Vec::new()
    }
}

fn fire_vblank_interrupt(emulator: &mut Emulator) {
    emulator.interrupts.flags |= 0x1;
}

fn lyc_check_enabled(emulator: &Emulator) -> bool {
    is_bit_set(emulator.gpu.registers.stat, STAT_INTERRUPT_LYC_CHECK_BIT) 
}

fn fire_stat_interrupt(emulator: &mut Emulator) {
    emulator.interrupts.flags |= 0x2;
}

fn update_mode(emulator: &mut Emulator, new_mode: u8) {
    emulator.gpu.mode = new_mode;

    let stat = (emulator.gpu.registers.stat & 0b11111100) | new_mode;
    emulator.gpu.registers.stat = stat;

    let fire_interrupt_on_mode_switch = (new_mode == OAM_MODE && is_bit_set(stat, OAM_MODE_STAT_SOURCE_BIT))
        || (new_mode == VBLANK_MODE && is_bit_set(stat, VBLANK_MODE_STAT_SOURCE_BIT))
        || (new_mode == HBLANK_MODE && is_bit_set(stat, HBLANK_MODE_STAT_SOURCE_BIT));

    if fire_interrupt_on_mode_switch {
        fire_stat_interrupt(emulator);
    }
}

fn compare_ly_and_lyc(emulator: &mut Emulator) {
    if emulator.gpu.registers.ly == emulator.gpu.registers.lyc {
        emulator.gpu.registers.stat = emulator.gpu.registers.stat | 0b00000100;
        
        if lyc_check_enabled(emulator) {
            fire_stat_interrupt(emulator);
        }
    }
    else {
        emulator.gpu.registers.stat = emulator.gpu.registers.stat & 0b11111011;
    }
}

pub fn step(emulator: &mut Emulator, mut render: impl FnMut(&Vec<u32>)) {
    emulator.gpu.mode_clock += emulator.cpu.clock.instruction_clock_cycles as u16;

    match emulator.gpu.mode {
        OAM_MODE => {
            if emulator.gpu.mode_clock >= OAM_TIME {
                emulator.gpu.sprite_buffer = collect_scanline_sprites(emulator);
                emulator.gpu.mode_clock = 0;
                update_mode(emulator, VRAM_MODE);
            }
        }
        VRAM_MODE => {
            if emulator.gpu.mode_clock >= VRAM_TIME {
                emulator.gpu.mode_clock = 0;
                update_mode(emulator, HBLANK_MODE);
                write_scanline(emulator);
            }
        }
        HBLANK_MODE => {
            if emulator.gpu.mode_clock >= HBLANK_TIME {
                if emulator.gpu.registers.ly == FRAME_SCANLINE_COUNT - VBLANK_SCANLINE_COUNT - 1 {
                    update_mode(emulator, VBLANK_MODE);
                    render(&emulator.gpu.frame_buffer);
                    fire_vblank_interrupt(emulator);
                }
                else {
                    update_mode(emulator, OAM_MODE);
                }

                emulator.gpu.registers.ly += 1;
                emulator.gpu.mode_clock = 0;

                compare_ly_and_lyc(emulator);
            }
        }
        VBLANK_MODE => {
            if emulator.gpu.mode_clock >= SCANLINE_RENDER_TIME {
                emulator.gpu.mode_clock = 0;
                emulator.gpu.registers.ly += 1;

                if emulator.gpu.registers.ly > FRAME_SCANLINE_COUNT - 1 {
                    emulator.gpu.registers.ly = 0;
                    update_mode(emulator, OAM_MODE);
                }

                compare_ly_and_lyc(emulator);
            }
        }
        _ => ()
    }    
}

#[cfg(test)]
mod tests;

mod colors;
mod constants;
mod line_addressing;
mod background;
mod window;
pub mod scanline;
pub mod sprites;
pub mod utils;