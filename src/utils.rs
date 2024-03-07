pub fn hexdump(buf: &[u8]) {
        uefi_services::print!("HEXDUMP");
        for (i, b) in buf.iter().enumerate() {
                if i & 0xf == 0 {
                        uefi_services::print!("\n{i:04x}:\t");
                }
                uefi_services::print!("{b:02x} ");
        }
        uefi_services::println!();
}
