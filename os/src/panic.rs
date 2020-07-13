use crate::sbi;
use core::panic::PanicInfo;

#[panic_handler]
fn on_panic(info: &PanicInfo) -> ! {
    println!("\x1b[1;31mpanic: '{}'\x1b[0m", info.message().unwrap());
    sbi::shutdown()
}