// QKZ TC MAX (KT Micro 31B2:0111) register access over HID interface 3, report 0x4B.
use hidapi::{HidApi, HidDevice};
use std::time::{Duration, Instant};

pub const VID: u16 = 0x31B2;
pub const PID: u16 = 0x0111;
const IFACE: i32 = 3;
const RPT: u8 = 0x4B;
const CMD_READ: u8 = 0x52;
const CMD_WRITE: u8 = 0x57;
const ACK: u8 = 0x03;

pub const REG_DAC_EQ_EN: u32 = 0x24;
pub const REG_DAC_EQ_BASE: u32 = 0x26;
pub const REG_DAC_GAIN: u32 = 0x66;
pub const BANDS: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Band {
    pub freq: u16, // Hz
    pub gain: f32, // dB, stored as i16 gain*10
    pub q: f32,    // stored as Q*1000
    pub kind: u16, // filter type, mapping unverified
}

impl Band {
    pub fn from_regs(a: u32, b: u32) -> Self {
        Band {
            freq: (a >> 16) as u16,
            gain: (a as u16 as i16) as f32 / 10.0,
            q: (b & 0xFFFF) as f32 / 1000.0,
            kind: (b >> 16) as u16,
        }
    }
    pub fn to_regs(self) -> (u32, u32) {
        let g = (self.gain * 10.0).round() as i16 as u16 as u32;
        let q = ((self.q * 1000.0).round() as u32).min(0xFFFF);
        (((self.freq as u32) << 16) | g, ((self.kind as u32) << 16) | q)
    }
}

pub struct Device {
    h: HidDevice,
}

impl Device {
    pub fn open() -> Result<Self, String> {
        let api = HidApi::new().map_err(|e| e.to_string())?;
        let info = api
            .device_list()
            .find(|d| d.vendor_id() == VID && d.product_id() == PID && d.interface_number() == IFACE)
            .ok_or("Perangkat (interface 3) tidak ditemukan")?;
        let h = info.open_device(&api).map_err(|e| e.to_string())?;
        Ok(Device { h })
    }

    fn transact(&self, tx: [u8; 11], cmd: u8) -> Result<[u8; 64], String> {
        self.h.write(&tx).map_err(|e| e.to_string())?;
        let t0 = Instant::now();
        let mut rx = [0u8; 64];
        while t0.elapsed() < Duration::from_millis(500) {
            let n = self.h.read_timeout(&mut rx, 100).map_err(|e| e.to_string())?;
            if n >= 11 && rx[0] == RPT && rx[5] == cmd && rx[1..5] == tx[1..5] {
                return Ok(rx);
            }
        }
        Err(format!(
            "timeout addr 0x{:02X}",
            u32::from_le_bytes([tx[1], tx[2], tx[3], tx[4]])
        ))
    }

    pub fn read(&self, addr: u32) -> Result<u32, String> {
        let a = addr.to_le_bytes();
        let tx = [RPT, a[0], a[1], a[2], a[3], CMD_READ, 0, 0, 0, 0, 0];
        let rx = self.transact(tx, CMD_READ)?;
        Ok(u32::from_le_bytes([rx[7], rx[8], rx[9], rx[10]]))
    }

    pub fn write(&self, addr: u32, val: u32) -> Result<(), String> {
        let a = addr.to_le_bytes();
        let v = val.to_le_bytes();
        let tx = [RPT, a[0], a[1], a[2], a[3], CMD_WRITE, 0, v[0], v[1], v[2], v[3]];
        let rx = self.transact(tx, CMD_WRITE)?;
        if rx[7] != ACK {
            return Err(format!("write 0x{:02X}: ACK 0x{:02X}", addr, rx[7]));
        }
        Ok(())
    }

    // ASCII stored little-endian across consecutive registers, NUL-terminated (e.g. firmware at 0x04).
    pub fn read_str(&self, start: u32, words: u32) -> Result<String, String> {
        let mut bytes = Vec::new();
        for r in start..start + words {
            bytes.extend_from_slice(&self.read(r)?.to_le_bytes());
        }
        let end = bytes.iter().position(|&c| c == 0).unwrap_or(bytes.len());
        Ok(String::from_utf8_lossy(&bytes[..end]).into_owned())
    }

    pub fn read_bands(&self) -> Result<[Band; BANDS], String> {
        let mut out = [Band {
            freq: 0,
            gain: 0.0,
            q: 0.707,
            kind: 0,
        }; BANDS];
        for (i, b) in out.iter_mut().enumerate() {
            let r = REG_DAC_EQ_BASE + i as u32 * 2;
            *b = Band::from_regs(self.read(r)?, self.read(r + 1)?);
        }
        Ok(out)
    }

    pub fn write_band(&self, i: usize, b: &Band) -> Result<(), String> {
        let r = REG_DAC_EQ_BASE + i as u32 * 2;
        let (a, bb) = b.to_regs();
        self.write(r, a)?;
        self.write(r + 1, bb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_factory_band() {
        let b = Band::from_regs(0x1388_0000, 0x0000_02C3);
        assert_eq!(
            b,
            Band {
                freq: 5000,
                gain: 0.0,
                q: 0.707,
                kind: 0
            }
        );
    }

    #[test]
    fn negative_gain_roundtrip() {
        let b = Band {
            freq: 60,
            gain: -6.5,
            q: 1.2,
            kind: 0,
        };
        let (a, bb) = b.to_regs();
        assert_eq!(a, (60 << 16) | 0xFFBF);
        assert_eq!(Band::from_regs(a, bb), b);
    }
}
