use crate::access_port::{
    AccessPortNumber,
    AccessPortError
};
use crate::access_port::consts::*;
use crate::dap_access::DAPAccess;

pub enum MemoryReadSize {
    U8 = CSW_SIZE8 as isize,
    U16 = CSW_SIZE16 as isize,
    U32 = CSW_SIZE32 as isize,
}

pub trait ToMemoryReadSize {
    fn to_alignment_mask() -> u32;
    fn to_memory_read_size() -> u32;
    fn to_result(value: u32) -> Self;
    fn to_input(value: &Self) -> u32;
}

impl ToMemoryReadSize for u32 {
    fn to_alignment_mask() -> u32 {
        0x3
    }

    fn to_memory_read_size() -> u32 {
        CSW_SIZE32
    }

    fn to_result(value: u32) -> Self {
        value
    }

    fn to_input(value: &Self) -> u32 {
        *value
    }
}

impl ToMemoryReadSize for u16 {
    fn to_alignment_mask() -> u32 {
        0x1
    }

    fn to_memory_read_size() -> u32 {
        CSW_SIZE16
    }

    fn to_result(value: u32) -> Self {
        value as u16
    }

    fn to_input(value: &Self) -> u32 {
        *value as u32
    }
}

impl ToMemoryReadSize for u8 {
    fn to_alignment_mask() -> u32 {
        0x0
    }

    fn to_memory_read_size() -> u32 {
        CSW_SIZE8
    }

    fn to_result(value: u32) -> Self {
        value as u8
    }

    fn to_input(value: &Self) -> u32 {
        *value as u32
    }
}

pub struct MemoryInterface {
    access_port: AccessPortNumber,
}

impl MemoryInterface {

    pub fn new(access_port: AccessPortNumber) -> Self {
        Self {
            access_port
        }
    }

    fn read_reg(&self, debug_port: &mut impl DAPAccess, addr: u32) -> Result<u32, AccessPortError> {
        debug_port.read_register(self.access_port, addr).or_else(|e| { println!("{:?}", e); Err(e) }).or_else(|_| Err(AccessPortError::ProbeError))
    }

    fn write_reg(&self, debug_port: &mut impl DAPAccess, addr: u32, data: u32) -> Result<(), AccessPortError> {
        debug_port.write_register(self.access_port, addr, data).or_else(|_| Err(AccessPortError::ProbeError))
    }

    pub fn read<S: ToMemoryReadSize>(&self, debug_port: &mut impl DAPAccess, addr: u32) -> Result<S, AccessPortError> {
        if (addr & S::to_alignment_mask()) == 0 {
            self.write_reg(debug_port, MEM_AP_CSW, CSW_VALUE | S::to_memory_read_size() as u32)?;
            self.write_reg(debug_port, MEM_AP_TAR, addr)?;
            let result = self.read_reg(debug_port, MEM_AP_DRW)?;
            Ok(S::to_result(result))
        } else {
            Err(AccessPortError::MemoryNotAligned)
        }
    }

    pub fn read_block_simple<S: ToMemoryReadSize>(&self, debug_port: &mut impl DAPAccess, addr: u32, data: &mut [S]) -> Result<(), AccessPortError> {
        if (addr & S::to_alignment_mask()) == 0 {
            let unit_size = std::mem::size_of::<S>() as u32;
            let len = data.len() as u32;
            self.write_reg(debug_port, MEM_AP_CSW, CSW_VALUE | S::to_memory_read_size() as u32)?;
            for offset in 0..len {
                let addr = addr + offset * unit_size;
                self.write_reg(debug_port, MEM_AP_TAR, addr)?;
                data[offset as usize] = S::to_result(self.read_reg(debug_port, MEM_AP_DRW)?);
            }
            Ok(())
        } else {
            Err(AccessPortError::MemoryNotAligned)
        }
    }

    pub fn read_block<S: ToMemoryReadSize + std::fmt::LowerHex + std::fmt::Debug>(
        &self,
        debug_port: &mut impl DAPAccess,
        addr: u32,
        data: &mut [S]
    ) -> Result<(), AccessPortError> {
        if (addr & S::to_alignment_mask()) == 0 {
            let unit_size = std::mem::size_of::<S>() as u32;
            let f = 4 / unit_size;
            let missing_words_at_start = (4 - addr & 0x3) / f;
            let missing_words_at_end = (data.len() as u32 - missing_words_at_start) % f;

            let len = (data.len() as u32 - missing_words_at_start - missing_words_at_end) / f;

            self.write_reg(debug_port, MEM_AP_CSW, CSW_VALUE | S::to_memory_read_size() as u32)?;
            for offset in 0..missing_words_at_start {
                let addr = addr + offset * unit_size;
                self.write_reg(debug_port, MEM_AP_TAR, addr)?;
                data[offset as usize] = S::to_result(self.read_reg(debug_port, MEM_AP_DRW)?);
            }

            self.write_reg(debug_port, MEM_AP_CSW, CSW_VALUE | CSW_SIZE32)?;
            for offset in 0..len {
                let addr = addr + missing_words_at_start * unit_size + offset * 4;
                self.write_reg(debug_port, MEM_AP_TAR, addr)?;
                let num_units = 4 / unit_size;
                let value = self.read_reg(debug_port, MEM_AP_DRW)?;
                for i in 0..num_units {
                    data[(missing_words_at_start + offset * f + i) as usize] = S::to_result(value >> (i * unit_size * 8));
                }
            }

            self.write_reg(debug_port, MEM_AP_CSW, CSW_VALUE | S::to_memory_read_size() as u32)?;
            for offset in 0..missing_words_at_end {
                let addr = addr + missing_words_at_start * unit_size + len * 4 + offset * unit_size;
                self.write_reg(debug_port, MEM_AP_TAR, addr)?;
                data[(missing_words_at_start + len * f + offset) as usize] = S::to_result(self.read_reg(debug_port, MEM_AP_DRW)?);
            }
            Ok(())
        } else {
            Err(AccessPortError::MemoryNotAligned)
        }
    }

    pub fn write<S: ToMemoryReadSize>(&self, debug_port: &mut impl DAPAccess, addr: u32, data: S) -> Result<(), AccessPortError> {
        if (addr & S::to_alignment_mask()) == 0 {
            self.write_reg(debug_port, MEM_AP_CSW, CSW_VALUE | S::to_memory_read_size())?;
            self.write_reg(debug_port, MEM_AP_TAR, addr)?;
            self.write_reg(debug_port, MEM_AP_DRW, S::to_input(&data))?;
            Ok(())
        } else {
            Err(AccessPortError::MemoryNotAligned)
        }
    }

    pub fn write_block<S: ToMemoryReadSize>(&self, debug_port: &mut impl DAPAccess, addr: u32, data: &[S]) -> Result<(), AccessPortError> {
        if (addr & S::to_alignment_mask()) == 0 {
            let len = data.len() as u32;
            let unit_size = std::mem::size_of::<S>() as u32;
            self.write_reg(debug_port, MEM_AP_CSW, CSW_VALUE | S::to_memory_read_size())?;
            for offset in 0..len {
                let addr = addr + offset * unit_size;
                self.write_reg(debug_port, MEM_AP_TAR, addr)?;
                self.write_reg(debug_port, MEM_AP_DRW, S::to_input(&data[offset as usize]))?;
            }
            Ok(())
        } else {
            Err(AccessPortError::MemoryNotAligned)
        }
    }
}

#[cfg(test)]
mod test {
    use super::MemoryInterface;
    use crate::dap_access::MockDAP;

    #[test]
    fn read_u32() {
        let mut mock = MockDAP::new();
        mock.data[0] = 0xEF;
        mock.data[1] = 0xBE;
        mock.data[2] = 0xAD;
        mock.data[3] = 0xDE;
        let mi = MemoryInterface::new(0x0);
        let read: Result<u32, _> = mi.read(&mut mock, 0);
        debug_assert!(read.is_ok());
        debug_assert_eq!(read.unwrap(), 0xDEADBEEF);
    }

    #[test]
    fn read_u16() {
        let mut mock = MockDAP::new();
        mock.data[0] = 0xEF;
        mock.data[1] = 0xBE;
        mock.data[2] = 0xAD;
        mock.data[3] = 0xDE;
        let mi = MemoryInterface::new(0x0);
        let read: Result<u16, _> = mi.read(&mut mock, 0);
        let read2: Result<u16, _> = mi.read(&mut mock, 2);
        debug_assert!(read.is_ok());
        debug_assert_eq!(read.unwrap(), 0xBEEF);
        debug_assert_eq!(read2.unwrap(), 0xDEAD);
    }

    #[test]
    fn read_u8() {
        let mut mock = MockDAP::new();
        mock.data[0] = 0xEF;
        mock.data[1] = 0xBE;
        mock.data[2] = 0xAD;
        mock.data[3] = 0xDE;
        let mi = MemoryInterface::new(0x0);
        let read: Result<u8, _> = mi.read(&mut mock, 0);
        let read2: Result<u8, _> = mi.read(&mut mock, 1);
        let read3: Result<u8, _> = mi.read(&mut mock, 2);
        let read4: Result<u8, _> = mi.read(&mut mock, 3);
        debug_assert!(read.is_ok());
        debug_assert_eq!(read.unwrap(), 0xEF);
        debug_assert_eq!(read2.unwrap(), 0xBE);
        debug_assert_eq!(read3.unwrap(), 0xAD);
        debug_assert_eq!(read4.unwrap(), 0xDE);
    }

    #[test]
    fn write_u32() {
        let mut mock = MockDAP::new();
        let mi = MemoryInterface::new(0x0);
        debug_assert!(mi.write(&mut mock, 0, 0xDEADBEEF as u32).is_ok());
        debug_assert_eq!(mock.data[0..4], [0xEF, 0xBE, 0xAD, 0xDE]);
    }

    #[test]
    fn write_u16() {
        let mut mock = MockDAP::new();
        let mi = MemoryInterface::new(0x0);
        debug_assert!(mi.write(&mut mock, 0, 0xBEEF as u16).is_ok());
        debug_assert!(mi.write(&mut mock, 2, 0xDEAD as u16).is_ok());
        debug_assert_eq!(mock.data[0..4], [0xEF, 0xBE, 0xAD, 0xDE]);
    }

    #[test]
    fn write_u8() {
        let mut mock = MockDAP::new();
        let mi = MemoryInterface::new(0x0);
        debug_assert!(mi.write(&mut mock, 0, 0xEF as u8).is_ok());
        debug_assert!(mi.write(&mut mock, 1, 0xBE as u8).is_ok());
        debug_assert!(mi.write(&mut mock, 2, 0xAD as u8).is_ok());
        debug_assert!(mi.write(&mut mock, 3, 0xDE as u8).is_ok());
        debug_assert_eq!(mock.data[0..4], [0xEF, 0xBE, 0xAD, 0xDE]);
    }

    #[test]
    fn read_block_u32() {
        let mut mock = MockDAP::new();
        mock.data[0] = 0xEF;
        mock.data[1] = 0xBE;
        mock.data[2] = 0xAD;
        mock.data[3] = 0xDE;
        mock.data[4] = 0xBE;
        mock.data[5] = 0xBA;
        mock.data[6] = 0xBA;
        mock.data[7] = 0xAB;
        let mi = MemoryInterface::new(0x0);
        let mut data = [0 as u32; 2];
        let read = mi.read_block(&mut mock, 0, &mut data);
        debug_assert!(read.is_ok());
        debug_assert_eq!(data, [0xDEADBEEF, 0xABBABABE]);
    }

    #[test]
    fn read_block_u16() {
        let mut mock = MockDAP::new();
        mock.data[0] = 0xEF;
        mock.data[1] = 0xBE;
        mock.data[2] = 0xAD;
        mock.data[3] = 0xDE;
        mock.data[4] = 0xBE;
        mock.data[5] = 0xBA;
        mock.data[6] = 0xBA;
        mock.data[7] = 0xAB;
        let mi = MemoryInterface::new(0x0);
        let mut data = [0 as u16; 4];
        let read = mi.read_block(&mut mock, 0, &mut data);
        debug_assert!(read.is_ok());
        debug_assert_eq!(data, [0xBEEF, 0xDEAD, 0xBABE, 0xABBA]);
    }

    #[test]
    fn read_block_u16_unaligned() {
        let mut mock = MockDAP::new();
        mock.data[2] = 0xEF;
        mock.data[3] = 0xBE;
        mock.data[4] = 0xAD;
        mock.data[5] = 0xDE;
        mock.data[6] = 0xBE;
        mock.data[7] = 0xBA;
        mock.data[8] = 0xBA;
        mock.data[9] = 0xAB;
        let mi = MemoryInterface::new(0x0);
        let mut data = [0 as u16; 4];
        let read = mi.read_block(&mut mock, 2, &mut data);
        debug_assert!(read.is_ok());
        debug_assert_eq!(data, [0xBEEF, 0xDEAD, 0xBABE, 0xABBA]);
    }

    #[test]
    fn read_block_u8() {
        let mut mock = MockDAP::new();
        mock.data[0] = 0xEF;
        mock.data[1] = 0xBE;
        mock.data[2] = 0xAD;
        mock.data[3] = 0xDE;
        mock.data[4] = 0xBE;
        mock.data[5] = 0xBA;
        mock.data[6] = 0xBA;
        mock.data[7] = 0xAB;
        let mi = MemoryInterface::new(0x0);
        let mut data = [0 as u8; 8];
        let read = mi.read_block(&mut mock, 0, &mut data);
        debug_assert!(read.is_ok());
        debug_assert_eq!(data, [0xEF, 0xBE, 0xAD, 0xDE, 0xBE, 0xBA, 0xBA ,0xAB]);
    }

    #[test]
    fn read_block_u8_unaligned() {
        let mut mock = MockDAP::new();
        mock.data[1] = 0xEF;
        mock.data[2] = 0xBE;
        mock.data[3] = 0xAD;
        mock.data[4] = 0xDE;
        mock.data[5] = 0xBE;
        mock.data[6] = 0xBA;
        mock.data[7] = 0xBA;
        mock.data[8] = 0xAB;
        let mi = MemoryInterface::new(0x0);
        let mut data = [0 as u8; 8];
        let read = mi.read_block(&mut mock, 1, &mut data);
        debug_assert!(read.is_ok());
        debug_assert_eq!(data, [0xEF, 0xBE, 0xAD, 0xDE, 0xBE, 0xBA, 0xBA ,0xAB]);
    }

    #[test]
    fn read_block_u8_unaligned2() {
        let mut mock = MockDAP::new();
        mock.data[3] = 0xEF;
        mock.data[4] = 0xBE;
        mock.data[5] = 0xAD;
        mock.data[6] = 0xDE;
        mock.data[7] = 0xBE;
        mock.data[8] = 0xBA;
        mock.data[9] = 0xBA;
        mock.data[10] = 0xAB;
        let mi = MemoryInterface::new(0x0);
        let mut data = [0 as u8; 8];
        let read = mi.read_block(&mut mock, 3, &mut data);
        debug_assert!(read.is_ok());
        debug_assert_eq!(data, [0xEF, 0xBE, 0xAD, 0xDE, 0xBE, 0xBA, 0xBA ,0xAB]);
    }

    #[test]
    fn write_block_u32() {
        let mut mock = MockDAP::new();
        let mi = MemoryInterface::new(0x0);
        debug_assert!(mi.write_block(&mut mock, 0, &([0xDEADBEEF, 0xABBABABE] as [u32; 2])).is_ok());
        debug_assert_eq!(mock.data[0..8], [0xEF, 0xBE, 0xAD, 0xDE, 0xBE, 0xBA, 0xBA ,0xAB]);
    }

    #[test]
    fn write_block_u16() {
        let mut mock = MockDAP::new();
        let mi = MemoryInterface::new(0x0);
        debug_assert!(mi.write_block(&mut mock, 0, &([0xBEEF, 0xDEAD, 0xBABE, 0xABBA] as [u16; 4])).is_ok());
        debug_assert_eq!(mock.data[0..8], [0xEF, 0xBE, 0xAD, 0xDE, 0xBE, 0xBA, 0xBA ,0xAB]);
    }

    #[test]
    fn write_block_u8() {
        let mut mock = MockDAP::new();
        let mi = MemoryInterface::new(0x0);
        debug_assert!(mi.write_block(&mut mock, 0, &([0xEF, 0xBE, 0xAD, 0xDE, 0xBE, 0xBA, 0xBA ,0xAB] as [u8; 8])).is_ok());
        debug_assert_eq!(mock.data[0..8], [0xEF, 0xBE, 0xAD, 0xDE, 0xBE, 0xBA, 0xBA ,0xAB]);
    }
}