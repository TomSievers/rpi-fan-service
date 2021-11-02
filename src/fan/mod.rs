use serde::Deserialize;
use std::fmt::Debug;
use rppal::gpio::{OutputPin, Result, Gpio};
use rppal::pwm::Pwm;
use std::cmp::Ordering;

#[derive(Deserialize, Debug)]
pub struct Fan {
    pin : Option<u8>,
    curve : std::vec::Vec<Curve>,
    update_rate : u16,
    hardware_pwm_channel : Option<u8>,
}

#[derive(Deserialize, Debug, Eq, Clone)]
pub struct Curve {
    percentage : u8,
    temperature : u8,
}

impl Ord for Curve {
    fn cmp(&self, other: &Self) -> Ordering {
        self.temperature.cmp(&other.temperature)
    }
}

impl PartialOrd for Curve {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Curve {
    fn eq(&self, other: &Self) -> bool {
        self.temperature == other.temperature
    }
}


#[derive(Debug)]
pub struct ControlledFan {
    pin : Option<OutputPin>,
    lut : [u8; 256],
    hardware_pwm : Option<Pwm>,
}

impl ControlledFan {
    pub fn software(pin : u8, curve : std::vec::Vec<Curve>) -> Result<ControlledFan>{
        let gpio = match Gpio::new() {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

        let real_pin = match gpio.get(pin) {
            Ok(p) => p.into_output_low(),
            Err(e) => return Err(e),
        };

        

        Ok(ControlledFan {
            pin : Some(real_pin),
            lut : ControlledFan::generate_lut(curve),
            hardware_pwm : None,
        })
    }

    pub fn hardware(channel : u8, curve : std::vec::Vec<Curve>) -> Result<ControlledFan> {

        let chan = match channel {
            0 => rppal::pwm::Channel::Pwm0,
            1 => rppal::pwm::Channel::Pwm1,
            _ => return Err(rppal::gpio::Error::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid PWM channel"))),
        };

        let pwm = match rppal::pwm::Pwm::with_frequency(chan, 20000.0f64, 0.0f64, rppal::pwm::Polarity::Normal, true) {
            Ok(v) => v,
            Err(e) => {
                match e {
                    rppal::pwm::Error::Io(error) => return Err(rppal::gpio::Error::Io(error)),
                }
            },
        };

        Ok(ControlledFan {
            pin : None,
            lut : ControlledFan::generate_lut(curve),
            hardware_pwm : Some(pwm),
        })
    }

    pub fn update(&mut self, temp : u8) -> Result<()> {
        let new_fan_speed = (self.lut[temp as usize] as f64) / 100.0f64;

        match self.pin.as_mut() {
            Some(v) => {
                return v.set_pwm_frequency(20000.0f64, new_fan_speed);
            },
            None => (),
        }

        match &self.hardware_pwm {
            Some(v) => {
                match v.set_duty_cycle(new_fan_speed) {
                    Ok(()) => return Ok(()),
                    Err(e) => {
                        match e {
                            rppal::pwm::Error::Io(error) => return Err(rppal::gpio::Error::Io(error)),
                        }
                    },
                }
            },
            None => (),
        }

        Ok(())
    }

    fn generate_lut(curve : std::vec::Vec<Curve>) -> [u8; 256] {
        let mut curve_index = 0;
        let mut lut : [u8; 256] = [0; 256];
        for (i, val) in lut.iter_mut().enumerate() {
            if curve_index < curve.len() && usize::from(curve[curve_index].temperature) < i {
                curve_index += 1;
            }

            if curve_index == 0 {
                *val = curve.first().unwrap().percentage;
            } else if curve_index >= curve.len() {
                *val = curve.last().unwrap().percentage;
            } else {
                let d_temp = (curve[curve_index-1].temperature as i16) - (curve[curve_index].temperature as i16);
                let d_speed = (curve[curve_index-1].percentage as i16) - (curve[curve_index].percentage as i16);
                let a = (d_speed as f32) / (d_temp as f32);
                let b = curve[curve_index-1].temperature as f32;
                *val = (a*(i as f32) - b).round() as u8;
            }
        }

        return lut;
    }
}

impl Fan {
    pub fn init(&self) -> Result<ControlledFan> {
        match self.pin {
            Some(v) => return ControlledFan::software(v, self.curve.clone()),
            None => (),
        }

        match self.hardware_pwm_channel {
            Some(v) => return ControlledFan::hardware(v, self.curve.clone()),
            None => return Err(rppal::gpio::Error::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Either pin or pwm channel needs to be present"))),
        }
        
    }

    pub fn update_rate(&self) -> u16 {
        return self.update_rate;
    }
}