pub fn load_m64(tas_file: String) -> Vec<u32> {
    if let Ok(m64_file) = std::fs::read(tas_file) {
        let signature = u32::from_le_bytes(m64_file[0..4].try_into().unwrap());
        let version = u32::from_le_bytes(m64_file[4..8].try_into().unwrap());
        let num_controllers = m64_file[0x15];
        let start_type = u16::from_le_bytes(m64_file[0x1c..0x1e].try_into().unwrap());

        let offset = if version == 3 {
            0x400
        } else if version == 2 || version == 1 {
            0x200
        } else {
            0
        };

        if signature == 0x1a34364d && offset != 0 && num_controllers == 1 && start_type == 2 {
            println!("TAS file loaded successfully");
            m64_file[offset..]
                .chunks_exact(4)
                .map(|chunk| u32::from_le_bytes(chunk.try_into().unwrap()))
                .collect()
        } else {
            eprintln!("could not load m64 TAS file");
            Vec::new()
        }
    } else {
        eprintln!("could not read m64 TAS file");
        Vec::new()
    }
}
