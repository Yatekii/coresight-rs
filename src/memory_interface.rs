
use crate::access_port::{
    AccessPortNumber,
    AccessPortError
};
use crate::access_port::consts::*;
use crate::dap_access::DAPAccess;

pub struct MemoryInterface {
    access_port: AccessPortNumber,
}

impl MemoryInterface {
    const auto_increment_page_size: u32 = 0x400;

    pub fn new(access_port: AccessPortNumber) -> Self {
        Self {
            access_port
        }
    }

    fn read_reg(&self, debug_port: &mut impl DAPAccess, addr: u32) -> Result<u32, AccessPortError> {
        debug_port.read_register(self.access_port, addr).or_else(|_| Err(AccessPortError::ProbeError))
    }

    fn write_reg(&self, debug_port: &mut impl DAPAccess, addr: u32, data: u32) -> Result<(), AccessPortError> {
        debug_port.write_register(self.access_port, addr, data).or_else(|_| Err(AccessPortError::ProbeError))
    }
    
    /// Write a single memory location.
    ///
    /// By default the transfer size is a word
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

    /// Read a memory location.
    ///
    /// By default, a word will be read.
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

    /// Write a single transaction's worth of aligned words.
    ///
    /// The transaction must not cross the MEM-AP's auto-increment boundary.
    fn write_block_u32(&mut self, _addr: u32, _data: &[u32]) -> Result<(), AccessPortError> {
        // TODO:
        Ok(())
    }

    /// Read a single transaction's worth of aligned words.
    ///
    /// The transaction must not cross the MEM-AP's auto-increment boundary.
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

// ================== MAYBE LATER ========================== //

// fn read_reg(&mut self, addr: u32) -> Result<u32, AccessPortError> {
//     let ap_regaddr = addr & APREG_MASK;
//     if ap_regaddr == MEM_AP_CSW && self._csw != -1 {
//         self._csw
//     } else {
//         AccessPort::read_reg(addr)
//     }
// }

// fn write_reg(&mut self, addr: u32, data: u32) -> Result<(), AccessPortError> {
//     let ap_regaddr = addr & APREG_MASK;

//     // Don't need to write CSW if it's not changing value.
//     if ap_regaddr == MEM_AP_CSW {
//         if data == self._csw {
//             return Ok(());
//         }
//         self._csw = data;
//     }
//     let result = AccessPort::write_reg(self, addr, data)
//                             .or_else(|e| {
//                                 if let DebugPortError::DebugProbeError = e {
//                                     if ap_regaddr == MEM_AP_CSW {
//                                         self._csw = -1;
//                                     }
//                                 }
//                                 e
//                             });
// }

// fn reset_did_occur(&mut self) {
//     // TODO use notifications to invalidate CSW cache.
//     self._csw = -1;
// }

// /// Write a single transaction's worth of aligned words.
// ///
// /// The transaction must not cross the MEM-AP's auto-increment boundary.
// fn write_block_u32(&mut self, addr: u32, data: &[u32]) -> Result<(), AccessPortError> {
//     if addr & 0x3 == 0 {
//         // put address in TAR
//         self.write_reg(MEM_AP_CSW, CSW_VALUE | CSW_SIZE32)?;
//         self.write_reg(MEM_AP_TAR, addr)?;
//         self.debug_probe.write_ap_multiple((self.access_port << APSEL_SHIFT) | MEM_AP_DRW, data)?;
//         Ok(())
//     } else {
//         Err(AccessPortError::MemoryNotAligned)
//     }
// }

// /// Read a single transaction's worth of aligned words.
// ///
// /// The transaction must not cross the MEM-AP's auto-increment boundary.
// fn read_block_u32(&mut self, addr: u32, data: &mut [u32]) -> Result<(), AccessPortError> {
//     if addr & 0x3 == 0 {
//         // put address in TAR
//         self.write_reg(MEM_AP_CSW, CSW_VALUE | CSW_SIZE32);
//         self.write_reg(MEM_AP_TAR, addr);
//         self.debug_probe.read_ap_multiple((self.access_port << APSEL_SHIFT) | MEM_AP_DRW, data)?;
//         Ok(())
//     } else {
//         Err(AccessPortError::MemoryNotAligned)
//     }
// }

// /// Write an aligned block of 32-bit words.
// fn write_memory_block32(&mut self, addr: u32, data: &[u32]) -> Result<(), AccessPortError> {
//     if addr & 0x3 == 0 {
//         let mut size = data.len() as u32;
//         let mut offset = 0;
//         while size > 0 {
//             let current_addr = addr + offset;
//             let mut n = Self::auto_increment_page_size - (current_addr + & (Self::auto_increment_page_size - 1));
//             if size * 4 < n {
//                 n = (size * 4) & 0xfffffffc;
//             }
//             self.write_block_u32(current_addr, &data[offset as usize..offset as usize + n as usize / 4]);
//             size -= n / 4;
//             offset += n;
//         }
//         Ok(())
//     } else {
//         Err(AccessPortError::MemoryNotAligned)
//     }
// }

// /// Read an aligned block of 32-bit words.
// fn read_memory_block32(&mut self, mut addr: u32, data: &mut [u32]) -> Result<(), AccessPortError> {
//     if addr & 0x3 == 0 {
//         let mut size = data.len() as u32;
//         let mut offset = 0;
//         while size > 0 {
//             let current_addr = addr + offset;
//             let mut n = Self::auto_increment_page_size - (current_addr & (Self::auto_increment_page_size - 1));
//             if size * 4 < n {
//                 n = (size * 4) & 0xFFFFFFFC;
//             }
//             self.read_block_u32(current_addr, &mut data[offset as usize..offset as usize + n as usize / 4])?;
//             size -= n / 4;
//             offset += n;
//         }
//         Ok(())
//     } else {
//         Err(AccessPortError::MemoryNotAligned)
//     }
// }