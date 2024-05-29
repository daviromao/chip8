use std::{fs::File, io::Read, process::Command};

#[macro_use]
mod macros;

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
        }
    }

    fn load_font(&mut self) {
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

    fn load_rom(&mut self, path: &str) {
        let mut file = File::open(path).expect("Erro when open ROM");
        file.read(&mut self.memory[0x200..Kilobytes!(4)])
            .expect("Erro when read");
        self.pc = 0x200;
    }
}

// 00E0 - CLS
fn clean_screen(chip: &mut ChipContext) {
    for (_, y) in chip.framebuffer.iter_mut().enumerate() {
        for pixel in y.iter_mut() {
            *pixel = false;
        }
    }
}

// 00EE - RET
fn return_sbr(chip: &mut ChipContext) {
    chip.sp -= 1;
    chip.pc = chip.stack[chip.sp as usize];
}

// 1nnn - JP addr
fn jump(chip: &mut ChipContext, nnn: u16) {
    chip.pc = nnn;
}

// 2nnn - CALL addr
fn call_sbr(chip: &mut ChipContext, nnn: u16) {
    chip.stack[chip.sp as usize] = chip.pc;
    chip.sp += 1;
    chip.pc = nnn;
}

// 3xkk - SE Vx, byte
fn skip_if(chip: &mut ChipContext, x: u8, kk: u8) {
    if chip.v[x as usize] == kk {
        chip.pc += 2;
    }
}

// 4xkk - SNE Vx, byte
fn skip_diff(chip: &mut ChipContext, x: u8, kk: u8) {
    if chip.v[x as usize] != kk {
        chip.pc += 2;
    }
}

// 5xy0 - SE Vx, Vy
fn skip_equals(chip: &mut ChipContext, x: u8, y: u8) {
    if chip.v[x as usize] == chip.v[y as usize] {
        chip.pc += 2;
    }
}

// 6xkk - LD Vx, byte
fn load(chip: &mut ChipContext, x: u8, kk: u8) {
    chip.v[x as usize] = kk;
}

// 7xkk - ADD Vx, byte
fn add(chip: &mut ChipContext, x: u8, kk: u8) {
    let sum = (chip.v[x as usize] as u16) + (kk as u16);
    chip.v[x as usize] = (sum & 0xFF) as u8;
}

// 8xy0 - LD Vx, Vy
fn load_vx_vy(chip: &mut ChipContext, x: u8, y: u8) {
    chip.v[x as usize] = chip.v[y as usize];
}

// 8xy1 - OR Vx, Vy
fn or(chip: &mut ChipContext, x: u8, y: u8) {
    chip.v[x as usize] |= chip.v[y as usize];
}

// 8xy2 - AND Vx, Vy
fn and(chip: &mut ChipContext, x: u8, y: u8) {
    chip.v[x as usize] &= chip.v[y as usize];
}

// 8xy3 - XOR Vx, Vy
fn xor(chip: &mut ChipContext, x: u8, y: u8) {
    chip.v[x as usize] ^= chip.v[y as usize];
}

// 8xy4 - ADD Vx, Vy
fn add_vx_vy(chip: &mut ChipContext, x: u8, y: u8) {
    let sum = (chip.v[x as usize] as u16) + (chip.v[y as usize] as u16);
    chip.v[x as usize] = (sum & 0xFF) as u8;
    chip.v[0xF] = if sum > 0xFF { 1 } else { 0 };
}

// 8xy5 - SUB Vx, Vy
fn sub(chip: &mut ChipContext, x: u8, y: u8) {
    chip.v[0xF] = if chip.v[x as usize] > chip.v[y as usize] {
        1
    } else {
        0
    };

    chip.v[x as usize] = chip.v[x as usize].wrapping_sub(chip.v[y as usize]);
}

// 8xy6 - SHR Vx Vy
fn shr(chip: &mut ChipContext, x: u8, _y: u8) {
    chip.v[0xF] = chip.v[x as usize] & 0x1;
    chip.v[x as usize] >>= 1;
}

// 8xy7 - SUBN Vx, Vy
fn subn(chip: &mut ChipContext, x: u8, y: u8) {
    chip.v[0xF] = if chip.v[y as usize] > chip.v[x as usize] {
        1
    } else {
        0
    };

    chip.v[x as usize] = chip.v[y as usize].wrapping_sub(chip.v[x as usize]);
}

// 8xyE - SHL Vx Vy
fn shl(chip: &mut ChipContext, x: u8, _y: u8) {
    chip.v[0xF] = (chip.v[x as usize] & 0x80) >> 7;
    chip.v[x as usize] <<= 1;
}

// 9xy0 - SNE Vx, Vy
fn sne(chip: &mut ChipContext, x: u8, y: u8) {
    if chip.v[x as usize] != chip.v[y as usize] {
        chip.pc += 2;
    }
}

// Annn - LD I, addr
fn set_index(chip: &mut ChipContext, nnn: u16) {
    chip.i = nnn;
}

// Bnnn - JP V0, addr
fn jump_v0(chip: &mut ChipContext, nnn: u16) {
    chip.pc = nnn + (chip.v[0] as u16);
}

// Cxkk - RND Vx, byte
fn rnd(chip: &mut ChipContext, x: u8, kk: u8) {
    let random = rand::random::<u8>();
    chip.v[x as usize] = random & kk;
}

// Dxyn - DRW Vx, Vy, nibble
fn draw(chip: &mut ChipContext, x: u8, y: u8, n: u8) {
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
            chip.framebuffer[x_temp][y_coord] = chip.framebuffer[x_temp][y_coord] ^ swap_pixel;
            x_temp = (x_temp + 1) % 64;
        }
        y_coord = (y_coord + 1) % 32;
    }
}

// Ex9E - SKP Vx
fn skip_key(chip: &mut ChipContext, x: u8) {
    if chip.v[x as usize] == 1 {
        chip.pc += 2;
    }
}

// ExA1 - SKNP Vx
fn skip_not_key(chip: &mut ChipContext, x: u8) {
    if chip.v[x as usize] == 0 {
        chip.pc += 2;
    }
}

// Fx07 - LD Vx, DT
fn load_vx_dt(chip: &mut ChipContext, x: u8) {
    chip.v[x as usize] = chip.dt;
}

// Fx0A - LD Vx, K
fn load_key(_chip: &mut ChipContext, _x: u8) {
    todo!();
}

// Fx15 - LD DT, Vx
fn load_dt_vx(chip: &mut ChipContext, x: u8) {
    chip.dt = chip.v[x as usize];
}

// Fx18 - LD ST, Vx
fn load_st_vx(chip: &mut ChipContext, x: u8) {
    chip.st = chip.v[x as usize];
}

// Fx1E - ADD I, Vx
fn add_i_vx(chip: &mut ChipContext, x: u8) {
    chip.i += chip.v[x as usize] as u16;
}

// Fx29 - LD F, Vx
fn load_f_vx(chip: &mut ChipContext, x: u8) {
    chip.i = (chip.v[x as usize] as u16) * 5;
}

// Fx33 - LD B, Vx
fn load_b_vx(chip: &mut ChipContext, x: u8) {
    chip.memory[chip.i as usize] = chip.v[x as usize] / 100;
    chip.memory[(chip.i + 1) as usize] = (chip.v[x as usize] / 10) % 10;
    chip.memory[(chip.i + 2) as usize] = chip.v[x as usize] % 10;
}

// Fx55 - LD [I], Vx
fn load_i_vx(chip: &mut ChipContext, x: u8) {
    for i in 0..=x {
        chip.memory[(chip.i + i as u16) as usize] = chip.v[i as usize];
    }
}

// Fx65 - LD Vx, [I]
fn load_vx_i(chip: &mut ChipContext, x: u8) {
    for i in 0..=x {
        chip.v[i as usize] = chip.memory[(chip.i + i as u16) as usize];
    }
}

fn clear_terminal() {
    if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", "cls"])
            .status()
            .expect("Failed to clear terminal");
    } else {
        Command::new("clear")
            .status()
            .expect("Failed to clear terminal");
    }
}

fn render(chip: &ChipContext) {
    clear_terminal();
    print!("▛");
    for _ in 0..64 {
        print!("▀")
    }
    println!("▜");
    for y in 0..32 {
        print!("▌");
        for x in 0..64 {
            print!("{}", if chip.framebuffer[x][y] { "▇" } else { " " });
        }
        print!("▐");
        println!();
    }
    print!("▙");
    for _ in 0..64 {
        print!("▄")
    }
    println!("▟");
    println!();
}

fn main() {
    let mut chip = ChipContext::new();
    chip.load_font();

    let mut args = std::env::args();
    args.next();
    let rom_path = args.next().expect("ROM path not provided");
    chip.load_rom(&rom_path);

    chip.pc = 0x200;

    loop {
        let byte1 = chip.memory[chip.pc as usize];
        let byte2 = chip.memory[(chip.pc + 1) as usize];
        let opcode = ((byte1 as u16) << 8) | byte2 as u16;
        chip.pc += 2;
        match opcode >> 12 {
            0x0 => match opcode & 0x00FF {
                0xE0 => clean_screen(&mut chip),
                0xEE => return_sbr(&mut chip),
                _ => panic!("Invalid opcode"),
            },
            0x1 => jump(&mut chip, opcode & 0x0FFF),
            0x2 => call_sbr(&mut chip, opcode & 0x0FFF),
            0x3 => skip_if(
                &mut chip,
                ((opcode & 0x0F00) >> 8) as u8,
                (opcode & 0x00FF) as u8,
            ),
            0x4 => skip_diff(
                &mut chip,
                ((opcode & 0x0F00) >> 8) as u8,
                (opcode & 0x00FF) as u8,
            ),
            0x5 => skip_equals(
                &mut chip,
                ((opcode & 0x0F00) >> 8) as u8,
                ((opcode & 0x00F0) >> 4) as u8,
            ),
            0x6 => load(
                &mut chip,
                ((opcode & 0x0F00) >> 8) as u8,
                (opcode & 0x00FF) as u8,
            ),
            0x7 => add(
                &mut chip,
                ((opcode & 0x0F00) >> 8) as u8,
                (opcode & 0x00FF) as u8,
            ),
            0x8 => match opcode & 0x000F {
                0x0 => load_vx_vy(
                    &mut chip,
                    ((opcode & 0x0F00) >> 8) as u8,
                    ((opcode & 0x00F0) >> 4) as u8,
                ),
                0x1 => or(
                    &mut chip,
                    ((opcode & 0x0F00) >> 8) as u8,
                    ((opcode & 0x00F0) >> 4) as u8,
                ),
                0x2 => and(
                    &mut chip,
                    ((opcode & 0x0F00) >> 8) as u8,
                    ((opcode & 0x00F0) >> 4) as u8,
                ),
                0x3 => xor(
                    &mut chip,
                    ((opcode & 0x0F00) >> 8) as u8,
                    ((opcode & 0x00F0) >> 4) as u8,
                ),
                0x4 => add_vx_vy(
                    &mut chip,
                    ((opcode & 0x0F00) >> 8) as u8,
                    ((opcode & 0x00F0) >> 4) as u8,
                ),
                0x5 => sub(
                    &mut chip,
                    ((opcode & 0x0F00) >> 8) as u8,
                    ((opcode & 0x00F0) >> 4) as u8,
                ),
                0x6 => shr(
                    &mut chip,
                    ((opcode & 0x0F00) >> 8) as u8,
                    ((opcode & 0x00F0) >> 4) as u8,
                ),
                0x7 => subn(
                    &mut chip,
                    ((opcode & 0x0F00) >> 8) as u8,
                    ((opcode & 0x00F0) >> 4) as u8,
                ),
                0xE => shl(
                    &mut chip,
                    ((opcode & 0x0F00) >> 8) as u8,
                    ((opcode & 0x00F0) >> 4) as u8,
                ),
                _ => panic!("Invalid opcode"),
            },
            0x9 => sne(
                &mut chip,
                ((opcode & 0x0F00) >> 8) as u8,
                ((opcode & 0x00F0) >> 4) as u8,
            ),
            0xA => set_index(&mut chip, opcode & 0x0FFF),
            0xB => jump_v0(&mut chip, opcode & 0x0FFF),
            0xC => rnd(
                &mut chip,
                ((opcode & 0x0F00) >> 8) as u8,
                (opcode & 0x00FF) as u8,
            ),
            0xD => draw(
                &mut chip,
                ((opcode & 0x0F00) >> 8) as u8,
                ((opcode & 0x00F0) >> 4) as u8,
                (opcode & 0x000F) as u8,
            ),
            0xE => match opcode & 0x00FF {
                0x9E => skip_key(&mut chip, ((opcode & 0x0F00) >> 8) as u8),
                0xA1 => skip_not_key(&mut chip, ((opcode & 0x0F00) >> 8) as u8),
                _ => panic!("Invalid opcode"),
            },
            0xF => match opcode & 0x00FF {
                0x07 => load_vx_dt(&mut chip, ((opcode & 0x0F00) >> 8) as u8),
                0x0A => load_key(&mut chip, ((opcode & 0x0F00) >> 8) as u8),
                0x15 => load_dt_vx(&mut chip, ((opcode & 0x0F00) >> 8) as u8),
                0x18 => load_st_vx(&mut chip, ((opcode & 0x0F00) >> 8) as u8),
                0x1E => add_i_vx(&mut chip, ((opcode & 0x0F00) >> 8) as u8),
                0x29 => load_f_vx(&mut chip, ((opcode & 0x0F00) >> 8) as u8),
                0x33 => load_b_vx(&mut chip, ((opcode & 0x0F00) >> 8) as u8),
                0x55 => load_i_vx(&mut chip, ((opcode & 0x0F00) >> 8) as u8),
                0x65 => load_vx_i(&mut chip, ((opcode & 0x0F00) >> 8) as u8),
                _ => panic!("Invalid opcode"),
            },
            _ => panic!("Invalid opcode"),
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
        render(&chip);
    }
}
