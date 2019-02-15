
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
    fn to_memory_read_size() -> u32;
    fn to_result(value: u32) -> Self;
    fn to_input(value: Self) -> u32;
}

impl ToMemoryReadSize for u32 {
    fn to_memory_read_size() -> u32 {
        CSW_SIZE32
    }

    fn to_result(value: u32) -> Self {
        value
    }

    fn to_input(value: Self) -> u32 {
        value
    }
}

impl ToMemoryReadSize for u16 {
    fn to_memory_read_size() -> u32 {
        CSW_SIZE16
    }

    fn to_result(value: u32) -> Self {
        value as u16
    }

    fn to_input(value: Self) -> u32 {
        value as u32
    }
}

impl ToMemoryReadSize for u8 {
    fn to_memory_read_size() -> u32 {
        CSW_SIZE8
    }

    fn to_result(value: u32) -> Self {
        value as u8
    }

    fn to_input(value: Self) -> u32 {
        value as u32
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
        if (addr & 0x3) == 0 {
            self.write_reg(debug_port, MEM_AP_CSW, CSW_VALUE | S::to_memory_read_size() as u32 )?;
            self.write_reg(debug_port, MEM_AP_TAR, addr)?;
            let result = self.read_reg(debug_port, MEM_AP_DRW)?;
            Ok(S::to_result(result))
        } else {
            Err(AccessPortError::MemoryNotAligned)
        }
    }

    pub fn write<S: ToMemoryReadSize>(&self, debug_port: &mut impl DAPAccess, addr: u32, data: S) -> Result<(), AccessPortError> {
        if (addr & 0x3) == 0 {
            self.write_reg(debug_port, MEM_AP_CSW, CSW_VALUE | S::to_memory_read_size())?;
            self.write_reg(debug_port, MEM_AP_TAR, addr)?;
            self.write_reg(debug_port, MEM_AP_DRW, S::to_input(data))?;
            Ok(())
        } else {
            Err(AccessPortError::MemoryNotAligned)
        }
    }
    
    /// Writes a single word to memory.
    /// 
    /// This needs the address to be aligned to 4 bytes.
    pub fn write_u32(&mut self, debug_port: &mut impl DAPAccess, addr: u32, data: u32) -> Result<(), AccessPortError> {
        if (addr & 0x3) == 0 {
            self.write_reg(debug_port, MEM_AP_CSW, CSW_VALUE | CSW_SIZE32)?;
            self.write_reg(debug_port, MEM_AP_TAR, addr)?;
            self.write_reg(debug_port, MEM_AP_DRW, data)?;
            Ok(())
        } else {
            Err(AccessPortError::MemoryNotAligned)
        }
    }

    /// Reads a single word from memory.
    /// 
    /// This needs the address to be aligned to 4 bytes.
    pub fn read_u32(&mut self, debug_port: &mut impl DAPAccess, addr: u32) -> Result<u32, AccessPortError>{
        if (addr & 0x3) == 0 {
            self.write_reg(debug_port, MEM_AP_CSW, CSW_VALUE | CSW_SIZE32)?;
            self.write_reg(debug_port, MEM_AP_TAR, addr)?;
            let result = self.read_reg(debug_port, MEM_AP_DRW)?;
            Ok(result)
        } else {
            Err(AccessPortError::MemoryNotAligned)
        }
    }

    /// Writes many words to a continuous ection of memory.
    /// 
    /// This needs the address to be aligned to 4 bytes.
    pub fn write_block_u32(&mut self, debug_port: &mut impl DAPAccess, addr: u32, data: &[u32]) -> Result<(), AccessPortError> {
        if (addr & 0x3) == 0 {
            let len = data.len() as u32;
            self.write_reg(debug_port, MEM_AP_CSW, CSW_VALUE | CSW_SIZE32)?;
            for offset in 0..len {
                let addr = addr + offset * 4;
                self.write_reg(debug_port, MEM_AP_TAR, addr)?;
                self.write_reg(debug_port, MEM_AP_DRW, data[offset as usize])?;
            }
            Ok(())
        } else {
            Err(AccessPortError::MemoryNotAligned)
        }
    }

    /// Reads many words from a continuous ection of memory.
    /// 
    /// This needs the address to be aligned to 4 bytes.
    pub fn read_block_u32(&mut self, debug_port: &mut impl DAPAccess, addr: u32, data: &mut [u32]) -> Result<(), AccessPortError> {
        if (addr & 0x3) == 0 {
            let len = data.len() as u32;
            self.write_reg(debug_port, MEM_AP_CSW, CSW_VALUE | CSW_SIZE32)?;
            for offset in 0..len {
                let addr = addr + offset * 4;
                self.write_reg(debug_port, MEM_AP_TAR, addr)?;
                data[offset as usize] = self.read_reg(debug_port, MEM_AP_DRW)?;
            }
            Ok(())
        } else {
            Err(AccessPortError::MemoryNotAligned)
        }
    }


}