mod models;

use linux_embedded_hal as hal;

use anyhow::Result;
use embedded_hal::blocking::delay::DelayMs;
use hal::spidev::{SpiModeFlags, SpidevOptions};
use hal::sysfs_gpio::Direction;
use hal::{Delay, Pin, Spidev};
use mfrc522::comm::{eh02::spi::SpiInterface, Interface};
use mfrc522::{Initialized, Mfrc522};

use security_iot::models::Led;

fn main() -> Result<()> {
    let mut led = Led;
    let mut delay = Delay;

    let mut spi = Spidev::open("/dev/spidev0.0").unwrap();
    let options = SpidevOptions::new()
        .max_speed_hz(1_000_000)
        .mode(SpiModeFlags::SPI_MODE_0)
        .build();
    spi.configure(&options).unwrap();

    // software-controlled chip select pin
    let pin = Pin::new(22);
    pin.export().unwrap();
    while !pin.is_exported() {}
    delay.delay_ms(1u32); // delay sometimes necessary because `is_exported()` returns too early?
    pin.set_direction(Direction::Out).unwrap();
    pin.set_value(1).unwrap();

    // The `with_nss` method provides a GPIO pin to the driver for software controlled chip select.
    let itf = SpiInterface::new(spi).with_nss(pin);
    let mut mfrc522 = Mfrc522::new(itf).init()?;

    let vers = mfrc522.version()?;

    println!("VERSION: 0x{:x}", vers);

    assert!(vers == 0x91 || vers == 0x92);


    loop {
        const CARD_UID: [u8; 4] = [34, 246, 178, 171];
        const TAG_UID: [u8; 4] = [128, 170, 179, 76];

        let reqa = mfrc522.reqa();

        match reqa {
            Ok(atqa) => {
                println!("Card detected");
                if let Ok(uid) = mfrc522.select(&atqa) {
                    println!("UID: {:?}", uid.as_bytes());
    
                    if uid.as_bytes() == CARD_UID {
                        led.off();
                        println!("CARD");
                    } else if uid.as_bytes() == TAG_UID {
                        led.on();
                        println!("TAG");
                    }
    
                    handle_authenticate(&mut mfrc522, &uid, |m| {
                        println!("read {:?}", m.mf_read(0)?);
                        println!("read {:?}", m.mf_read(1)?);
                        println!("read {:?}", m.mf_read(2)?);
                        println!("read {:?}", m.mf_read(3)?);
                        Ok(())
                    })
                    .ok();
                }
            },
            // Todo: Handler errors when its not a timeout error.
            Err(_) => {},
        }

        delay.delay_ms(10u32);
    }
}

fn handle_authenticate<E, COMM: Interface<Error = E>, F>(
    mfrc522: &mut Mfrc522<COMM, Initialized>,
    uid: &mfrc522::Uid,
    action: F,
) -> Result<()>
where
    F: FnOnce(&mut Mfrc522<COMM, Initialized>) -> Result<()>,
    E: std::fmt::Debug + std::marker::Sync + std::marker::Send + 'static,
{
    println!("Authenticating...");

    // Use *default* key, this should work on new/empty cards
    let key = [0xFF; 6];
    if mfrc522.mf_authenticate(uid, 1, &key).is_ok() {
        action(mfrc522)?;
    } else {
        println!("Could not authenticate");
    }

    mfrc522.hlta()?;
    mfrc522.stop_crypto1()?;
    Ok(())
}
