use std::{
    fs::File,
    io::Read,
    time::{Duration, Instant},
};

use sdl2::{
    event::Event, keyboard::Keycode, pixels::Color, rect::Rect, render::Canvas, video::Window,
};

#[macro_use]
mod macros;

const SCALE: u32 = 10;
const SLEEP_TIME: u64 = 2;

struct ChipContext {
    // Memory
    memory: [u8; Kilobytes!(4)],
    // Registers
    v: [u8; 16],

    // Timer
    dt: u8,
    st: u8,
    pc: u16,
    sp: u8,
    i: u16,

    // Stack
    stack: [u16; 16],

    // Display buffer on/off
    framebuffer: [[bool; 32]; 64],

    // Keys
    keyboard: [bool; 16],
}

impl ChipContext {
    fn new() -> Self {
        Self {
            memory: [0; Kilobytes!(4)],
            v: [0; 16],
            st: 0,
            dt: 0,
            pc: 0,
            sp: 0,
            i: 0,
            stack: [0; 16],
            framebuffer: [[false; 32]; 64],
            keyboard: [false; 16],
        }
    }

    fn intialize(&mut self) {
        self._load_rom();
        self._load_font();
    }

    fn _load_font(&mut self) {
        self.memory[0x50..0xA0].copy_from_slice(&[
            0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
            0x20, 0x60, 0x20, 0x20, 0x70, // 1
            0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
            0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
            0x90, 0x90, 0xF0, 0x10, 0x10, // 4
            0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
            0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
            0xF0, 0x10, 0x20, 0x40, 0x40, // 7
            0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
            0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
            0xF0, 0x90, 0xF0, 0x90, 0x90, // A
            0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
            0xF0, 0x80, 0x80, 0x80, 0xF0, // C
            0xE0, 0x90, 0x90, 0x90, 0xE0, // D
            0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
            0xF0, 0x80, 0xF0, 0x80, 0x80, // F
        ]);
    }

    fn _load_rom_from_path(&mut self, path: &str) {
        let mut file = File::open(path).expect("Erro when open ROM");
        file.read(&mut self.memory[0x200..Kilobytes!(4)])
            .expect("Erro when read");
        self.pc = 0x200;
    }

    fn _load_rom(&mut self) {
        let mut args = std::env::args();
        args.next();
        let rom_path = args.next();

        match rom_path {
            Some(path) => self._load_rom_from_path(&path),
            None => (),
        }
    }
}

enum Instruction {
    ClearScreen(),                // 00E0 - CLS
    Return(),                     // 00EE - RET
    Jump(u16),                    // 1nnn - JP addr
    Call(u16),                    // 2nnn - CALL addr
    SkipEqualByte(u8, u8),        // 3xkk - SE Vx, byte
    SkipNotEqualByte(u8, u8),     // 4xkk - SNE Vx, byte
    SkipEqualRegister(u8, u8),    // 5xy0 - SE Vx, Vy
    LoadByte(u8, u8),             // 6xkk - LD Vx, byte
    AddByte(u8, u8),              // 7xkk - ADD Vx, byte
    LoadRegister(u8, u8),         // 8xy0 - LD Vx, Vy
    Or(u8, u8),                   // 8xy1 - OR Vx, Vy
    And(u8, u8),                  // 8xy2 - AND Vx, Vy
    Xor(u8, u8),                  // 8xy3 - XOR Vx, Vy
    AddRegister(u8, u8),          // 8xy4 - ADD Vx, Vy
    SubRegister(u8, u8),          // 8xy5 - SUB Vx, Vy
    ShiftRight(u8),               // 8xy6 - SHR Vx {, Vy}
    SubRegisterNB(u8, u8),        // 8xy7 - SUBN Vx, Vy
    ShiftLeft(u8),                // 8xyE - SHL Vx {, Vy}
    SkipNotEqualRegister(u8, u8), // 9xy0 - SNE Vx, Vy
    LoadIndex(u16),               // Annn - LD I, addr
    JumpV0(u16),                  // Bnnn - JP V0, addr
    RandomByte(u8, u8),           // Cxkk - RND Vx, byte
    Draw(u8, u8, u8),             // Dxyn - DRW Vx, Vy, nibble
    SkipKeyPressed(u8),           // Ex9E - SKP Vx
    SkipKeyNotPressed(u8),        // ExA1 - SKNP Vx
    LoadDelayTimer(u8),           // Fx07 - LD Vx, DT
    LoadAwaitKey(u8),             // Fx0A - LD Vx, K
    SetDelayTimer(u8),            // Fx15 - LD DT, Vx
    SetSoundTimer(u8),            // Fx18 - LD ST, Vx
    AddIndex(u8),                 // Fx1E - ADD I, Vx
    LoadSprite(u8),               // Fx29 - LD F, Vx
    LoadBCD(u8),                  // Fx33 - LD B, Vx
    StoreRegisters(u8),           // Fx55 - LD [I], Vx
    LoadRegisters(u8),            // Fx65 - LD Vx, [I]
}

fn fetch(chip: &mut ChipContext) -> u16 {
    let byte1 = chip.memory[chip.pc as usize] as u16;
    let byte2 = chip.memory[(chip.pc + 1) as usize] as u16;
    let opcode = (byte1 << 8) | byte2;
    chip.pc += 2;

    opcode
}

fn decode(opcode: u16) -> Instruction {
    // four hex of opcode
    let first_hex = (opcode) >> 12;
    let second_hex = (opcode & 0x0F00) >> 8;
    let third_hex = (opcode & 0x00F0) >> 4;
    let fourth_hex = opcode & 0x000F;

    let nnn = (second_hex << 8) | (third_hex << 4) | fourth_hex;
    let n = fourth_hex as u8;
    let x = second_hex as u8;
    let y = third_hex as u8;
    let kk = ((third_hex << 4) | fourth_hex) as u8;

    match (first_hex, second_hex, third_hex, fourth_hex) {
        (0, 0, 0xE, 0) => Instruction::ClearScreen(),
        (0, 0, 0xE, 0xE) => Instruction::Return(),
        (1, _, _, _) => Instruction::Jump(nnn),
        (2, _, _, _) => Instruction::Call(nnn),
        (3, _, _, _) => Instruction::SkipEqualByte(x, kk),
        (4, _, _, _) => Instruction::SkipNotEqualByte(x, kk),
        (5, _, _, 0) => Instruction::SkipEqualRegister(x, y),
        (6, _, _, _) => Instruction::LoadByte(x, kk),
        (7, _, _, _) => Instruction::AddByte(x, kk),
        (8, _, _, 0) => Instruction::LoadRegister(x, y),
        (8, _, _, 1) => Instruction::Or(x, y),
        (8, _, _, 2) => Instruction::And(x, y),
        (8, _, _, 3) => Instruction::Xor(x, y),
        (8, _, _, 4) => Instruction::AddRegister(x, y),
        (8, _, _, 5) => Instruction::SubRegister(x, y),
        (8, _, _, 6) => Instruction::ShiftRight(x),
        (8, _, _, 7) => Instruction::SubRegisterNB(x, y),
        (8, _, _, 0xE) => Instruction::ShiftLeft(x),
        (9, _, _, 0) => Instruction::SkipNotEqualRegister(x, y),
        (0xA, _, _, _) => Instruction::LoadIndex(nnn),
        (0xB, _, _, _) => Instruction::JumpV0(nnn),
        (0xC, _, _, _) => Instruction::RandomByte(x, kk),
        (0xD, _, _, _) => Instruction::Draw(x, y, n),
        (0xE, _, 9, 0xE) => Instruction::SkipKeyPressed(x),
        (0xE, _, 0xA, 1) => Instruction::SkipKeyNotPressed(x),
        (0xF, _, 0, 7) => Instruction::LoadDelayTimer(x),
        (0xF, _, 0, 0xA) => Instruction::LoadAwaitKey(x),
        (0xF, _, 1, 5) => Instruction::SetDelayTimer(x),
        (0xF, _, 1, 8) => Instruction::SetSoundTimer(x),
        (0xF, _, 1, 0xE) => Instruction::AddIndex(x),
        (0xF, _, 2, 9) => Instruction::LoadSprite(x),
        (0xF, _, 3, 3) => Instruction::LoadBCD(x),
        (0xF, _, 5, 5) => Instruction::StoreRegisters(x),
        (0xF, _, 6, 5) => Instruction::LoadRegisters(x),
        (_, _, _, _) => panic!("Unknown opcode: {:#X}", opcode),
    }
}

fn execute(chip: &mut ChipContext, instruction: Instruction) {
    match instruction {
        Instruction::ClearScreen() => {
            for y in chip.framebuffer.iter_mut() {
                for pixel in y.iter_mut() {
                    *pixel = false;
                }
            }
        }
        Instruction::Return() => {
            chip.sp -= 1;
            chip.pc = chip.stack[chip.sp as usize];
        }
        Instruction::Jump(addr) => {
            chip.pc = addr;
        }
        Instruction::Call(addr) => {
            chip.stack[chip.sp as usize] = chip.pc;
            chip.sp += 1;
            chip.pc = addr;
        }
        Instruction::SkipEqualByte(x, byte) => {
            if chip.v[x as usize] == byte {
                chip.pc += 2;
            }
        }
        Instruction::SkipNotEqualByte(x, byte) => {
            if chip.v[x as usize] != byte {
                chip.pc += 2;
            }
        }
        Instruction::SkipEqualRegister(x, y) => {
            if chip.v[x as usize] == chip.v[y as usize] {
                chip.pc += 2;
            }
        }
        Instruction::LoadByte(x, byte) => {
            chip.v[x as usize] = byte;
        }
        Instruction::AddByte(x, byte) => {
            chip.v[x as usize] = chip.v[x as usize].wrapping_add(byte);
        }
        Instruction::LoadRegister(x, y) => {
            chip.v[x as usize] = chip.v[y as usize];
        }
        Instruction::Or(x, y) => {
            chip.v[x as usize] |= chip.v[y as usize];
        }
        Instruction::And(x, y) => {
            chip.v[x as usize] &= chip.v[y as usize];
        }
        Instruction::Xor(x, y) => {
            chip.v[x as usize] ^= chip.v[y as usize];
        }
        Instruction::AddRegister(x, y) => {
            let sum = (chip.v[x as usize] as u16) + (chip.v[y as usize] as u16);
            chip.v[x as usize] = (sum & 0xFF) as u8;
            chip.v[0xF] = if sum > 0xFF { 1 } else { 0 };
        }
        Instruction::SubRegister(x, y) => {
            chip.v[0xF] = if chip.v[x as usize] > chip.v[y as usize] {
                1
            } else {
                0
            };

            chip.v[x as usize] = chip.v[x as usize].wrapping_sub(chip.v[y as usize]);
        }
        Instruction::ShiftRight(x) => {
            chip.v[0xF] = chip.v[x as usize] & 0x1;
            chip.v[x as usize] >>= 1;
        }
        Instruction::SubRegisterNB(x, y) => {
            chip.v[0xF] = if chip.v[y as usize] > chip.v[x as usize] {
                1
            } else {
                0
            };

            chip.v[x as usize] = chip.v[y as usize].wrapping_sub(chip.v[x as usize]);
        }
        Instruction::ShiftLeft(x) => {
            chip.v[0xF] = (chip.v[x as usize] & 0x80) >> 7;
            chip.v[x as usize] <<= 1;
        }
        Instruction::SkipNotEqualRegister(x, y) => {
            if chip.v[x as usize] != chip.v[y as usize] {
                chip.pc += 2;
            }
        }
        Instruction::LoadIndex(addr) => {
            chip.i = addr;
        }
        Instruction::JumpV0(addr) => {
            chip.pc = addr + (chip.v[0] as u16);
        }
        Instruction::RandomByte(x, byte) => {
            let random = rand::random::<u8>();
            chip.v[x as usize] = random & byte;
        }
        Instruction::Draw(x, y, n) => {
            let x_coord: usize = (chip.v[x as usize] % 64) as usize;
            let mut y_coord: usize = (chip.v[y as usize] % 32) as usize;
            chip.v[0xF] = 0;
            for offset in 0..n {
                let mut x_temp = x_coord;
                for bitset in (0..8).rev() {
                    let swap_pixel: bool =
                        ((chip.memory[(chip.i + (offset as u16)) as usize] >> bitset) & 1) != 0;
                    if chip.framebuffer[x_temp][y_coord] && swap_pixel {
                        chip.v[0xF] = 1;
                    }
                    chip.framebuffer[x_temp][y_coord] =
                        chip.framebuffer[x_temp][y_coord] ^ swap_pixel;
                    x_temp = (x_temp + 1) % 64;
                }
                y_coord = (y_coord + 1) % 32;
            }
        }
        Instruction::SkipKeyPressed(x) => {
            if chip.keyboard[chip.v[x as usize] as usize] {
                chip.pc += 2;
            }
        }
        Instruction::SkipKeyNotPressed(x) => {
            if !chip.keyboard[chip.v[x as usize] as usize] {
                chip.pc += 2;
            }
        }
        Instruction::LoadDelayTimer(x) => {
            chip.v[x as usize] = chip.dt;
        }
        Instruction::LoadAwaitKey(x) => {
            let mut pressed = false;

            for i in 0..chip.keyboard.len() {
                if chip.keyboard[i] {
                    chip.v[x as usize] = i as u8;
                    pressed = true;
                    break;
                }
            }

            if !pressed {
                chip.pc -= 2
            };
        }
        Instruction::SetDelayTimer(x) => {
            chip.dt = chip.v[x as usize];
        }
        Instruction::SetSoundTimer(x) => {
            chip.st = chip.v[x as usize];
        }
        Instruction::AddIndex(x) => {
            chip.i += chip.v[x as usize] as u16;
        }
        Instruction::LoadSprite(x) => {
            chip.i = 0x50 + (chip.v[x as usize] as u16) * 5;
        }
        Instruction::LoadBCD(x) => {
            chip.memory[chip.i as usize] = chip.v[x as usize] / 100;
            chip.memory[(chip.i + 1) as usize] = (chip.v[x as usize] / 10) % 10;
            chip.memory[(chip.i + 2) as usize] = chip.v[x as usize] % 10;
        }
        Instruction::StoreRegisters(x) => {
            for i in 0..=x {
                chip.memory[(chip.i + i as u16) as usize] = chip.v[i as usize];
            }
        }
        Instruction::LoadRegisters(x) => {
            for i in 0..=x {
                chip.v[i as usize] = chip.memory[(chip.i + i as u16) as usize];
            }
        }
    }
}

fn render(chip: &ChipContext, canvas: &mut Canvas<Window>) {
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();

    canvas.set_draw_color(Color::RGB(255, 255, 255));

    for y in (0..32).map(|x| x as u32) {
        for x in (0..64).map(|y| y as u32) {
            if chip.framebuffer[x as usize][y as usize] {
                let rect = Rect::new((x * SCALE) as i32, (y * SCALE) as i32, SCALE, SCALE);
                canvas.fill_rect(rect).unwrap();
            }
        }
    }
    canvas.present();
}

fn key2btn(key: Keycode) -> Option<usize> {
    match key {
        Keycode::Num1 => Some(0x1),
        Keycode::Num2 => Some(0x2),
        Keycode::Num3 => Some(0x3),
        Keycode::Num4 => Some(0xC),
        Keycode::Q => Some(0x4),
        Keycode::W => Some(0x5),
        Keycode::E => Some(0x6),
        Keycode::R => Some(0xD),
        Keycode::A => Some(0x7),
        Keycode::S => Some(0x8),
        Keycode::D => Some(0x9),
        Keycode::F => Some(0xE),
        Keycode::Z => Some(0xA),
        Keycode::X => Some(0x0),
        Keycode::C => Some(0xB),
        Keycode::V => Some(0xF),
        _ => None,
    }
}

fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("Chip 8 - Emulator", 64 * SCALE, 32 * SCALE)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump()?;

    let mut chip = ChipContext::new();
    chip.intialize();

    let mut last_update = Instant::now();
    let clock = Duration::from_millis(1000 / 60);

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown {
                    keycode: Some(key), ..
                } => {
                    if let Some(k) = key2btn(key) {
                        chip.keyboard[k as usize] = true;
                    }
                }
                Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    if let Some(k) = key2btn(key) {
                        chip.keyboard[k as usize] = false;
                    }
                }
                _ => (),
            }
        }

        let opcode = fetch(&mut chip);
        let instruction = decode(opcode);
        execute(&mut chip, instruction);

        render(&chip, &mut canvas);

        if last_update.elapsed() >= clock {
            chip.dt = if chip.dt > 0 { chip.dt - 1 } else { 0 };
            chip.st = if chip.st > 0 { chip.st - 1 } else { 0 };
            last_update = Instant::now();
        }

        std::thread::sleep(std::time::Duration::from_millis(SLEEP_TIME));
    }
    Ok(())
}
