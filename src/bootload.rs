use hal::stm32;
use stm32f4xx_hal as hal;

// This can be any number, it's only used to determine if we should
// boot up in bootloader mode instead of running normally.
const MAGIC_BOOTLOADER_NUMBER: u32 = 131981;

// Which backup register we should read/write the magic number to.
// The STM32F411 has 20 backup registers, each of which is 32-bits wide.
const BACKUP_REGISTER_INDEX: usize = 0;

// The location of the bootloader firmware in system memory.
// Consult your STM32's datasheet or Application Note AN2602
// for this value.
const BOOTLOADER_FIRMWARE_MEMORY_LOCATION: u32 = 0x1FFF0000;

pub fn request_bootloader() -> ! {
    let dp = unsafe { stm32::Peripherals::steal() };

    enable_backup_domain(&dp);
    write_to_backup_register(MAGIC_BOOTLOADER_NUMBER, &dp);
    disable_backup_domain(&dp);

    cortex_m::peripheral::SCB::sys_reset();
}

pub fn jump_to_bootloader_if_requested(dp: &stm32::Peripherals) {
    let magic_num: u32 = read_backup_register(dp);

    if magic_num == MAGIC_BOOTLOADER_NUMBER {
        enable_backup_domain(&dp);
        write_to_backup_register(0, dp);
        disable_backup_domain(&dp);

        unsafe {
            cortex_m::asm::bootload(BOOTLOADER_FIRMWARE_MEMORY_LOCATION as *const u32);
        }
    }
}

fn read_backup_register(dp: &stm32::Peripherals) -> u32 {
    let rtc = &dp.RTC;
    rtc.bkpr[BACKUP_REGISTER_INDEX].read().bkp().bits()
}

fn write_to_backup_register(val: u32, dp: &stm32::Peripherals) {
    let rtc = &dp.RTC;
    rtc.bkpr[BACKUP_REGISTER_INDEX].write(|w| w.bkp().bits(val));
}

fn enable_backup_domain(dp: &hal::stm32::Peripherals) {
    let pwr = &dp.PWR;
    let rcc = &dp.RCC;

    // Enable the power interface clock by setting the PWREN bits in the RCC_APB1ENR register
    rcc.apb1enr.write(|w| w.pwren().bit(true));

    // Stall the pipeline to work around erratum 2.1.13 (DM00037591)
    cortex_m::asm::dsb();

    // Set the DBP bit in the Section 5.4.1 to enable access to the backup domain
    pwr.cr.write(|w| w.dbp().bit(true));

    // Enable the RTC clock by programming the RTCEN [15] bit in the Section 7.3.20: RCC Backup domain control register (RCC_BDCR)
    rcc.bdcr.write(|w| w.rtcen().bit(true));
}

fn disable_backup_domain(dp: &stm32::Peripherals) {
    dp.PWR.cr.write(|w| w.dbp().bit(false));
}
