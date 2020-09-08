pub mod error;

use bitvec::prelude::{bitvec, BitVec, Bits, LittleEndian};
use embedded_hal::blocking::spi::Write;
use embedded_hal::digital::v2::OutputPin;
use error::{Error, Result};
use std::marker::PhantomData;
use typenum::{Unsigned, U12, U16, U24};

pub struct TLC59xxx<SPI, LAT, WORD, CHANNELS> {
    spi: SPI,
    lat: LAT,
    shift_register: BitVec,
    phantom: PhantomData<(WORD, CHANNELS)>,
}

pub type TLC5947<SPI, LAT> = TLC59xxx<SPI, LAT, U12, U24>;

impl<SPI: Write<u8>, LAT: OutputPin> TLC5947<SPI, LAT> {
    /// Returns TLC59xxx driver with 24 channels & 12-bit words
    ///
    /// # Arguments
    ///
    /// * `spi` The embedded-hal spi device
    /// * `lat` An embedded-hal pin device, this is toggled once data has finished being written to the register
    /// * `chain_size` The amount of devices chained together
    pub fn new(spi: SPI, lat: LAT, chain_size: usize) -> TLC5947<SPI, LAT> {
        Self::new_device(spi, lat, chain_size)
    }
}

pub type TLC59711<SPI, LAT> = TLC59xxx<SPI, LAT, U16, U12>;

impl<SPI: Write<u8>, LAT: OutputPin> TLC59711<SPI, LAT> {
    /// Returns TLC59xxx driver with 12 channels & 16-bit words
    ///
    /// # Arguments
    ///
    /// * `spi` The embedded-hal spi device
    /// * `lat` An embedded-hal pin device, this is toggled once data has finished being written to the register
    /// * `chain_size` The amount of devices chained together
    pub fn new(spi: SPI, lat: LAT, chain_size: usize) -> TLC59711<SPI, LAT> {
        Self::new_device(spi, lat, chain_size)
    }
}

impl<SPI, LAT, WORD, CHANNELS> TLC59xxx<SPI, LAT, WORD, CHANNELS>
where
    SPI: Write<u8>,
    LAT: OutputPin,
    WORD: Unsigned,
    CHANNELS: Unsigned,
{
    fn new_device(spi: SPI, lat: LAT, chain_size: usize) -> TLC59xxx<SPI, LAT, WORD, CHANNELS> {
        TLC59xxx {
            spi,
            lat,
            shift_register: bitvec![0; CHANNELS::to_usize() * chain_size * WORD::to_usize()],
            phantom: PhantomData,
        }
    }

    /// Sets a channel to a given frequency
    ///
    /// # Arguments
    ///
    /// * `channel` The channel to change. This must be no greater than the total amount of channels in all chained devices
    /// * `val` The frequency to set the channel to. This must be no greater than what can be contained in the word size
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Embedded-hal device setup
    /// let tlc = TLC5947::new(spi, lat, 2);
    /// tlc.set_pwm(2, 4096); //Set to max for the tlc5947
    /// tlc.set_pwm(32, 512); //Set on second chained device
    /// tlc.write();
    /// ```
    pub fn set_pwm(&mut self, channel: usize, val: u16) {
        assert!((val as usize) < 2usize.pow(WORD::to_u32()));
        assert!(channel < (self.shift_register.len() / WORD::to_usize()));

        let end = self.shift_register.len() - channel * WORD::to_usize();
        let start = end - WORD::to_usize();
        let mut new_val = val.as_bitslice::<LittleEndian>()[..WORD::to_usize()].iter();

        for x in start..end {
            self.shift_register.set(x, new_val.next().unwrap());
        }
    }

    /// Helper function for rgb leds, sets 3 adjacent channels using set_pwm
    ///
    /// # Arguments
    ///
    /// * `light` The channel offset by multiple of 3
    /// * `val` A triplet of channel frequencies
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Embedded-hal device setup
    /// let tlc = TLC5947::new(spi, lat, 1);
    /// tlc.set_rgb(2, (4096, 128, 2048)); //This changes channels 6, 7, 8
    /// tlc.write();
    /// ```
    pub fn set_rgb(&mut self, light: usize, rgb: (u16, u16, u16)) {
        let light = light * 3;
        self.set_pwm(light, rgb.0);
        self.set_pwm(light + 1, rgb.1);
        self.set_pwm(light + 2, rgb.2);
    }

    /// Writes current register to the device
    pub fn write(&mut self) -> Result<()> {
        self.spi
            .write(&self.shift_register.as_slice())
            .map_err(|_| Error::Spi)?;

        self.lat.set_high().map_err(|_| Error::Lat)?;
        self.lat.set_low().map_err(|_| Error::Lat)?;
        Ok(())
    }

    /// Destroys the device, returning embedded-hal components
    pub fn destroy(self) -> (SPI, LAT) {
        (self.spi, self.lat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitvec::prelude::BitStore;
    use embedded_hal_mock::{
        pin::{Mock as PinMock, State as PinState, Transaction as PinTransaction},
        spi::{Mock as SpiMock, Transaction as SpiTransaction},
    };
    use rand::{
        distributions::{Distribution, Uniform},
        thread_rng,
    };

    fn test_configuration_random<WORD, CHANNELS>(
        chain_size: usize,
    ) -> (TLC59xxx<SpiMock, PinMock, WORD, CHANNELS>, BitVec)
    where
        WORD: Unsigned,
        CHANNELS: Unsigned,
    {
        let uniform = Uniform::from(0..2u64.pow(WORD::to_u32()));
        let mut rng = thread_rng();

        let mut shift_register =
            BitVec::with_capacity(WORD::to_usize() * CHANNELS::to_usize() * chain_size);

        for _ in 0..CHANNELS::to_usize() * chain_size {
            let random = uniform.sample(&mut rng);
            for x in 0..WORD::to_u8() {
                shift_register.push(random.get_at(x.into()));
            }
        }
        let shift_register_rev: BitVec = shift_register.clone().into_iter().rev().collect();
        let spi_expectation = [SpiTransaction::write(shift_register_rev.into_vec())];

        let pin_expectation: [PinTransaction; 2] = [
            PinTransaction::set(PinState::High),
            PinTransaction::set(PinState::Low),
        ];

        let spi = SpiMock::new(&spi_expectation);
        let lat = PinMock::new(&pin_expectation);
        let tlc = TLC59xxx {
            spi,
            lat,
            shift_register: bitvec![0; WORD::to_usize() * CHANNELS::to_usize() * chain_size],
            phantom: PhantomData,
        };

        (tlc, shift_register)
    }

    #[test]
    fn write_47() -> Result<()> {
        let (mut tlc, array): (TLC5947<_, _>, BitVec) = test_configuration_random(1);

        for (pos, val) in array.chunks(12).enumerate() {
            tlc.set_pwm(pos, val.iter().fold(0, |acc, bit| (acc << 1) | bit as u16));
        }
        tlc.write()?;

        Ok(())
    }

    #[test]
    fn rgb_47() -> Result<()> {
        let (mut tlc, array): (TLC5947<_, _>, BitVec) = test_configuration_random(1);

        for (pos, val) in array.chunks(36).enumerate() {
            let val: Vec<u16> = val
                .chunks(12)
                .map(|b| b.iter().fold(0, |acc, bit| (acc << 1) | bit as u16))
                .collect();
            tlc.set_rgb(pos, (val[0], val[1], val[2]));
        }
        tlc.write()?;

        Ok(())
    }

    #[test]
    fn chained_47_512() -> Result<()> {
        let (mut tlc, array): (TLC5947<_, _>, BitVec) = test_configuration_random(512);

        for (pos, val) in array.chunks(36).enumerate() {
            let val: Vec<u16> = val
                .chunks(12)
                .map(|b| b.iter().fold(0, |acc, bit| (acc << 1) | bit as u16))
                .collect();
            tlc.set_rgb(pos, (val[0], val[1], val[2]));
        }
        tlc.write()?;

        Ok(())
    }

    #[should_panic]
    #[test]
    fn pwm_oor_47() {
        let array = vec![0; (12 * 24) / 8];
        let spi_expectation = [SpiTransaction::write(array)];

        let pin_expectation: [PinTransaction; 2] = [
            PinTransaction::set(PinState::High),
            PinTransaction::set(PinState::Low),
        ];

        let spi = SpiMock::new(&spi_expectation);
        let pin = PinMock::new(&pin_expectation);
        let mut tlc = TLC5947::new(spi, pin, 1);

        tlc.set_pwm(1, 0b1_0000_0000_0000);
    }

    #[test]
    fn write_711() -> Result<()> {
        let (mut tlc, array): (TLC59711<_, _>, BitVec) = test_configuration_random(1);

        for (pos, val) in array.chunks(16).enumerate() {
            tlc.set_pwm(pos, val.iter().fold(0, |acc, bit| (acc << 1) | bit as u16));
        }
        tlc.write()?;

        Ok(())
    }

    #[test]
    fn rgb_711() -> Result<()> {
        let (mut tlc, array): (TLC59711<_, _>, BitVec) = test_configuration_random(1);

        for (pos, val) in array.chunks(48).enumerate() {
            let val: Vec<u16> = val
                .chunks(16)
                .map(|b| b.iter().fold(0, |acc, bit| (acc << 1) | bit as u16))
                .collect();
            tlc.set_rgb(pos, (val[0], val[1], val[2]));
        }
        tlc.write()?;

        Ok(())
    }

    #[test]
    fn chained_711_512() -> Result<()> {
        let (mut tlc, array): (TLC59711<_, _>, BitVec) = test_configuration_random(512);

        for (pos, val) in array.chunks(48).enumerate() {
            let val: Vec<u16> = val
                .chunks(16)
                .map(|b| b.iter().fold(0, |acc, bit| (acc << 1) | bit as u16))
                .collect();
            tlc.set_rgb(pos, (val[0], val[1], val[2]));
        }
        tlc.write()?;

        Ok(())
    }
}
