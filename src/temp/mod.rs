use regex::Regex;

pub fn get_temperature() -> Result<f32, std::io::Error> {
    match std::fs::read_to_string("/sys/class/thermal/thermal_zone0/temp") {
        Ok(temp_string) => {
            let reg = match Regex::new(r"[0-9]+") {
                Ok(val) => val,
                Err(_) => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Regex failed")),
            };

            let temp_string_clean = match reg.find(temp_string.as_str()) {
                Some(val) => val.as_str(),
                None => return Err(std::io::Error::new(std::io::ErrorKind::Other, "Regex failed")),
            };

            let parsed_temp : u32  = match temp_string_clean.parse::<u32>() {
                Ok(val) => val,
                Err(e) => {
                    println!("Failed: {:?}", e);
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Cannot convert result to int"));
                },
            };

            Ok((parsed_temp as f32) / 1000.0f32)
        },
        Err(e) => Err(e),
    }
}