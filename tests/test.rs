#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        fs,
        path::Path,
        rc::Rc,
        sync::mpsc::channel,
        time::{Duration, SystemTime},
    };

    use gamuboy::{
        config::Config, gameboy::GameBoy, joypad_events_handler, lcd, mode::Mode, saver, stereo,
    };

    const ROMS_PATH: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/roms");

    fn test_rom_dmg(component_name: &str, rom_name: &str, timeout: Duration) {
        test_rom(component_name, rom_name, timeout, Mode::DMG);
    }

    fn test_rom_cgb(component_name: &str, rom_name: &str, timeout: Duration) {
        test_rom(component_name, rom_name, timeout, Mode::CGB);
    }

    fn test_rom(component_name: &str, rom_name: &str, timeout: Duration, mode: Mode) {
        let rom = fs::read(
            Path::new(ROMS_PATH)
                .join(component_name)
                .join(rom_name)
                .to_str()
                .unwrap(),
        )
        .unwrap();

        let path_buf = Path::new(ROMS_PATH)
            .join(component_name)
            .join(rom_name.to_owned() + ".expected.txt");
        let expected_path = path_buf.to_str().unwrap();
        let expected_result = fs::read_to_string(expected_path);
        let is_update = expected_result.is_err();
        let expected = expected_result.unwrap_or("".to_owned());

        if is_update {
            println!("Update mode");
        }

        let (_, rx) = channel();
        let cfg = &Config {
            mode,
            rom,
            headless_mode: false,
            bootrom: None,
            log_file_path: None,
        };

        let output = Rc::new(RefCell::new("".to_owned()));

        let mut test_gb = GameBoy::new(
            cfg,
            lcd::Fake::new(output.clone()),
            stereo::Fake,
            joypad_events_handler::Fake,
            saver::Fake,
            &rx,
        );

        let start = SystemTime::now();
        while SystemTime::now()
            .duration_since(start)
            .expect("oopsy computing duration")
            <= timeout
        {
            test_gb.step();
            if !is_update && *output.borrow() == expected {
                return;
            }
        }

        if is_update {
            fs::write(expected_path, output.borrow().as_bytes().iter().as_slice())
                .expect("failed to write to file");
            return;
        }

        assert_eq!(expected, *output.borrow());
    }

    #[test]
    fn test_blargg_roms_cpu_instrs_01_special() {
        test_rom_dmg(
            "blargg/cpu_instrs",
            "01-special.gb",
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_blargg_roms_cpu_instrs_02_interrupts() {
        test_rom_dmg(
            "blargg/cpu_instrs",
            "02-interrupts.gb",
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_blargg_roms_cpu_instrs_03_op_sp_hl() {
        test_rom_dmg(
            "blargg/cpu_instrs",
            "03-op sp,hl.gb",
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_blargg_roms_cpu_instrs_04_op_r_imm() {
        test_rom_dmg(
            "blargg/cpu_instrs",
            "04-op r,imm.gb",
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_blargg_roms_cpu_instrs_05_op_rp() {
        test_rom_dmg("blargg/cpu_instrs", "05-op rp.gb", Duration::from_secs(30));
    }

    #[test]
    fn test_blargg_roms_cpu_instrs_06_ld_r_r() {
        test_rom_dmg("blargg/cpu_instrs", "06-ld r,r.gb", Duration::from_secs(30));
    }

    #[test]
    fn test_blargg_roms_cpu_instrs_07_jr_jp_call_ret_rst() {
        test_rom_dmg(
            "blargg/cpu_instrs",
            "07-jr,jp,call,ret,rst.gb",
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_blargg_roms_cpu_instrs_08_misc_instsr() {
        test_rom_dmg(
            "blargg/cpu_instrs",
            "08-misc instrs.gb",
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_blargg_roms_cpu_instrs_09_op_r_r() {
        test_rom_dmg("blargg/cpu_instrs", "09-op r,r.gb", Duration::from_secs(30));
    }

    #[test]
    fn test_blargg_roms_cpu_instrs_10_bit_ops() {
        test_rom_dmg(
            "blargg/cpu_instrs",
            "10-bit ops.gb",
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_blargg_roms_cpu_instrs_11_op_a_hl() {
        test_rom_dmg(
            "blargg/cpu_instrs",
            "11-op a,(hl).gb",
            Duration::from_secs(40),
        );
    }

    #[test]
    fn test_blargg_roms_instr_timing() {
        test_rom_dmg(
            "blargg/instr_timing",
            "instr_timing.gb",
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_blargg_roms_interrupt_time() {
        test_rom_cgb(
            "blargg/interrupt_time",
            "interrupt_time.gb",
            Duration::from_secs(10),
        );
    }

    #[test]
    fn test_blargg_roms_mem_timing_01_read_timing() {
        test_rom_dmg(
            "blargg/mem_timing",
            "01-read_timing.gb",
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_blargg_roms_mem_timing_02_write_timing() {
        test_rom_dmg(
            "blargg/mem_timing",
            "02-write_timing.gb",
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_blargg_roms_mem_timing_03_modify_timing() {
        test_rom_dmg(
            "blargg/mem_timing",
            "03-modify_timing.gb",
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_blargg_roms_mem_timing_2_01_read_timing() {
        test_rom_dmg(
            "blargg/mem_timing-2",
            "01-read_timing.gb",
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_blargg_roms_mem_timing_2_02_write_timing() {
        test_rom_dmg(
            "blargg/mem_timing-2",
            "02-write_timing.gb",
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_blargg_roms_mem_timing_2_03_modify_timing() {
        test_rom_dmg(
            "blargg/mem_timing-2",
            "03-modify_timing.gb",
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_blargg_roms_dmg_sound_01_registers() {
        test_rom_dmg(
            "blargg/dmg_sound",
            "01-registers.gb",
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_blargg_roms_dmg_sound_02_len_ctr() {
        test_rom_dmg("blargg/dmg_sound", "02-len ctr.gb", Duration::from_secs(30));
    }

    #[test]
    fn test_blargg_roms_dmg_sound_03_trigger() {
        test_rom_dmg("blargg/dmg_sound", "03-trigger.gb", Duration::from_secs(40));
    }

    #[test]
    fn test_blargg_roms_dmg_sound_04_sweep() {
        test_rom_dmg("blargg/dmg_sound", "04-sweep.gb", Duration::from_secs(30));
    }

    #[test]
    fn test_blargg_roms_dmg_sound_05_sweep_details() {
        test_rom_dmg(
            "blargg/dmg_sound",
            "05-sweep details.gb",
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_blargg_roms_dmg_sound_06_overflow_on_trigger() {
        test_rom_dmg(
            "blargg/dmg_sound",
            "06-overflow on trigger.gb",
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_blargg_roms_dmg_sound_07_len_sweep_period_sync() {
        test_rom_dmg(
            "blargg/dmg_sound",
            "07-len sweep period sync.gb",
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_blargg_roms_dmg_sound_08_len_str_during_power() {
        test_rom_dmg(
            "blargg/dmg_sound",
            "08-len ctr during power.gb",
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_blargg_roms_dmg_sound_09_wave_read_while_on() {
        test_rom_dmg(
            "blargg/dmg_sound",
            "09-wave read while on.gb",
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_blargg_roms_dmg_sound_10_wave_trigger_while_on() {
        test_rom_dmg(
            "blargg/dmg_sound",
            "10-wave trigger while on.gb",
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_blargg_roms_dmg_sound_11_regs_after_power() {
        test_rom_dmg(
            "blargg/dmg_sound",
            "11-regs after power.gb",
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_blargg_roms_dmg_sound_12_wave_write_while_on() {
        test_rom_dmg(
            "blargg/dmg_sound",
            "12-wave write while on.gb",
            Duration::from_secs(30),
        );
    }

    #[test]
    fn test_mooneye_roms_mbc1_bits_bank1() {
        test_rom_dmg("mooneye/mbc1", "bits_bank1.gb", Duration::from_secs(15));
    }

    #[test]
    fn test_mooneye_roms_mbc1_bits_bank2() {
        test_rom_dmg("mooneye/mbc1", "bits_bank2.gb", Duration::from_secs(15));
    }

    #[test]
    fn test_mooneye_roms_mbc1_bits_mode() {
        test_rom_dmg("mooneye/mbc1", "bits_mode.gb", Duration::from_secs(15));
    }

    #[test]
    fn test_mooneye_roms_mbc1_bits_ramg() {
        test_rom_dmg("mooneye/mbc1", "bits_ramg.gb", Duration::from_secs(15));
    }

    #[test]
    fn test_mooneye_roms_mbc1_ram_64kb() {
        test_rom_dmg("mooneye/mbc1", "ram_64kb.gb", Duration::from_secs(15));
    }

    #[test]
    fn test_mooneye_roms_mbc1_ram_256kb() {
        test_rom_dmg("mooneye/mbc1", "ram_256kb.gb", Duration::from_secs(15));
    }

    #[test]
    fn test_mooneye_roms_mbc1_rom_1mb() {
        test_rom_dmg("mooneye/mbc1", "rom_1Mb.gb", Duration::from_secs(15));
    }

    #[test]
    fn test_mooneye_roms_mbc1_rom_2mb() {
        test_rom_dmg("mooneye/mbc1", "rom_2Mb.gb", Duration::from_secs(15));
    }

    #[test]
    fn test_mooneye_roms_mbc1_rom_4mb() {
        test_rom_dmg("mooneye/mbc1", "rom_4Mb.gb", Duration::from_secs(15));
    }

    #[test]
    fn test_mooneye_roms_mbc1_rom_8mb() {
        test_rom_dmg("mooneye/mbc1", "rom_8Mb.gb", Duration::from_secs(15));
    }

    #[test]
    fn test_mooneye_roms_mbc1_rom_16mb() {
        test_rom_dmg("mooneye/mbc1", "rom_16Mb.gb", Duration::from_secs(15));
    }

    #[test]
    fn test_mooneye_roms_mbc1_rom_512kb() {
        test_rom_dmg("mooneye/mbc1", "rom_512kb.gb", Duration::from_secs(15));
    }

    #[test]
    fn test_mooneye_roms_mbc2_bits_ramg() {
        test_rom_dmg("mooneye/mbc2", "bits_ramg.gb", Duration::from_secs(10));
    }

    #[test]
    fn test_mooneye_roms_mbc2_bits_romb() {
        test_rom_dmg("mooneye/mbc2", "bits_romb.gb", Duration::from_secs(10));
    }

    #[test]
    fn test_mooneye_roms_mbc2_bits_unused() {
        test_rom_dmg("mooneye/mbc2", "bits_unused.gb", Duration::from_secs(10));
    }

    #[test]
    fn test_mooneye_roms_mbc2_ram() {
        test_rom_dmg("mooneye/mbc2", "ram.gb", Duration::from_secs(10));
    }

    #[test]
    fn test_mooneye_roms_mbc2_rom_1mb() {
        test_rom_dmg("mooneye/mbc2", "rom_1Mb.gb", Duration::from_secs(10));
    }

    #[test]
    fn test_mooneye_roms_mbc2_rom_2mb() {
        test_rom_dmg("mooneye/mbc2", "rom_2Mb.gb", Duration::from_secs(10));
    }

    #[test]
    fn test_mooneye_roms_mbc2_rom_512kb() {
        test_rom_dmg("mooneye/mbc2", "rom_512kb.gb", Duration::from_secs(10));
    }

    #[test]
    fn test_mooneye_roms_mbc5_rom_512kb() {
        test_rom_dmg("mooneye/mbc5", "rom_512kb.gb", Duration::from_secs(10));
    }

    #[test]
    fn test_mooneye_roms_mbc5_rom_1mb() {
        test_rom_dmg("mooneye/mbc5", "rom_1Mb.gb", Duration::from_secs(10));
    }

    #[test]
    fn test_mooneye_roms_mbc5_rom_2mb() {
        test_rom_dmg("mooneye/mbc5", "rom_2Mb.gb", Duration::from_secs(10));
    }

    #[test]
    fn test_mooneye_roms_mbc5_rom_4mb() {
        test_rom_dmg("mooneye/mbc5", "rom_4Mb.gb", Duration::from_secs(10));
    }

    #[test]
    fn test_mooneye_roms_mbc5_rom_8mb() {
        test_rom_dmg("mooneye/mbc5", "rom_8Mb.gb", Duration::from_secs(10));
    }

    #[test]
    fn test_mooneye_roms_mbc5_rom_16mb() {
        test_rom_dmg("mooneye/mbc5", "rom_16Mb.gb", Duration::from_secs(10));
    }

    #[test]
    fn test_mooneye_roms_mbc5_rom_32mb() {
        test_rom_dmg("mooneye/mbc5", "rom_32Mb.gb", Duration::from_secs(10));
    }

    #[test]
    fn test_mooneye_roms_mbc5_rom_64mb() {
        test_rom_dmg("mooneye/mbc5", "rom_64Mb.gb", Duration::from_secs(10));
    }
}
