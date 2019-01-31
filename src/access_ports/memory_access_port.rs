use dbg_probe::debug_probe::DebugProbeContainer;
use crate::access_port::AccessPortNumber;
use crate::access_port::NewAccessPort;
use crate::access_port::AccessPort;

use crate::access_port::consts;

pub enum AccessPortError {
    MemoryNotAligned,
}

pub struct MemoryAccessPort {
    access_port: AccessPortNumber,
}

impl MemoryAccessPort {
    pub fn new(self, debug_probe: &mut DebugProbeContainer, access_port: AccessPortNumber) {
        self.access_port = access_port;
        
        // Cached CSW value.
        self._csw = -1;

        // Default to the smallest size supported by all targets.
        // A size smaller than the supported size will decrease performance
        // due to the extra address writes, but will not create any
        // read/write errors.
        self.auto_increment_page_size = 0x400;
        
        // TODO:
        // Ask the probe for an accelerated memory interface for this AP. If it provides one,
        // then bind our memory interface APIs to its methods. Otherwise use our standard
        // memory interface based on AP register accesses.
        // let memory_interface = self.dp.link.get_memory_interface_for_ap(self.ap_num);
        // if memoryInterface is not None:
        //     logging.debug("Using accelerated memory access interface")
        //     self.write_memory = memoryInterface.write_memory
        //     self.read_memory = memoryInterface.read_memory
        //     self.write_memory_block32 = memoryInterface.write_memory_block32
        //     self.read_memory_block32 = memoryInterface.read_memory_block32
        // else:
        //     self.write_memory = self._write_memory
        //     self.read_memory = self._read_memory
        //     self.write_memory_block32 = self._write_memory_block32
        //     self.read_memory_block32 = self._read_memory_block32
    }

    fn read_reg(&mut self, addr: u32) -> Result<u32, AccessPortError> {
        let ap_regaddr = addr & APREG_MASK;
        if ap_regaddr == MEM_AP_CSW && self._csw != -1 {
            self._csw
        } else {
            AccessPort::read_reg(addr)
        }
    }

    fn write_reg(&mut self, addr: u32, data: u32) -> Result<(), AccessPortError> {
        let ap_regaddr = addr & APREG_MASK;

        // Don't need to write CSW if it's not changing value.
        if ap_regaddr == MEM_AP_CSW {
            if data == self._csw {
                return Ok(());
            }
            self._csw = data;
        }
        let result = AccessPort::write_reg(self, addr, data)
                                .or_else(|e| {
                                    if let DebugPortError::DebugProbeError = e {
                                        if ap_regaddr == MEM_AP_CSW {
                                            self._csw = -1;
                                        }
                                    }
                                    e
                                });
    }
    
    fn reset_did_occur(&mut self) {
        // TODO use notifications to invalidate CSW cache.
        self._csw = -1;
    }

    /// Write a single transaction's worth of aligned words.
    ///
    /// The transaction must not cross the MEM-AP's auto-increment boundary.
    fn write_block32(&mut self, addr: u32, data: &[u32]) -> Result<(), AccessPortError> {
        if addr & 0x3 == 0 {
            // put address in TAR
            self.write_reg(MEM_AP_CSW, CSW_VALUE | CSW_SIZE32)?;
            self.write_reg(MEM_AP_TAR, addr)?;
            self.debug_probe.write_ap_multiple((self.access_port << APSEL_SHIFT) | MEM_AP_DRW, data)?;
            Ok(())
        } else {
            Err(AccessPortError::MemoryNotAligned)
        }
    }

    /// Read a single transaction's worth of aligned words.
    ///
    /// The transaction must not cross the MEM-AP's auto-increment boundary.
    fn read_block32(&mut self, addr: u32, data: &mut [u32]) -> Result<(), AccessPortError> {
        if addr & 0x3 == 0 {
            // put address in TAR
            self.write_reg(MEM_AP_CSW, CSW_VALUE | CSW_SIZE32);
            self.write_reg(MEM_AP_TAR, addr);
            self.debug_probe.read_ap_multiple((self.access_port << APSEL_SHIFT) | MEM_AP_DRW, data)?;
            Ok(())
        } else {
            Err(AccessPortError::MemoryNotAligned)
        }
    }

    fn handle_error(&mut self, error: DebugProbeError) {
        self.debug_probe.handle_error(error);
        self.csw = -1;
    }
}

impl MemoryInterface for MemoryAccessPort {
    /// Write a single memory location.
    ///
    /// By default the transfer size is a word
    fn write_memory(&mut self, addr: u32, data: u32) -> Result<(), AccessPortError> {
        if (addr & 0x3) == 0 {
            self.write_reg(MEM_AP_CSW, CSW_VALUE | CSW_SIZE32)?;
            self.write_reg(MEM_AP_TAR, addr)?;
            self.write_reg(MEM_AP_DRW, data)?;
            Ok(())
        } else {
            Err(AccessPortError::MemoryNotAligned)
        }
    }

    /// Read a memory location.
    ///
    /// By default, a word will be read.
    fn read_memory(&mut self, addr: u32) -> Result<u32, AccessPortError>{
        if (addr & 0x3) == 0 {
            self.write_reg(MEM_AP_CSW, CSW_VALUE | CSW_SIZE32)?;
            self.write_reg(MEM_AP_TAR, addr)?;
            let mut result = self.read_reg(MEM_AP_DRW, data)?;
            Ok(result)
        } else {
            Err(AccessPortError::MemoryNotAligned)
        }
    }

    /// Write an aligned block of 32-bit words.
    fn write_memory_block32(&mut self, addr: u32, data: &[u32]) -> Result<(), AccessPortError> {
        if addr & 0x3 == 0 {
            let size = data.len();
            let mut offset = 0;
            while size > 0 {
                let current_addr = (addr + offset);
                let mut n = self.auto_increment_page_size - (current_addr + & (self.auto_increment_page_size - 1));
                if size * 4 < n {
                    n = (size * 4) & 0xfffffffc;
                }
                self.write_block32(current_addr, data[offset..n / 4]);
                size -= n / 4;
                offset += n;
            }
            Ok(())
        } else {
            Err(AccessPortError::MemoryNotAligned)
        }
    }

    /// Read an aligned block of 32-bit words.
    fn read_memory_block32(&mut self, mut addr: u32, data: &mut [u32]) -> Result<(), AccessPortError> {
        if addr & 0x3 == 0 {
            let mut size = data.len();
            let mut offset = 0;
            while size > 0 {
                let current_addr = (addr + offset);
                let mut n = self.auto_increment_page_size - (current_addr & (self.auto_increment_page_size - 1));
                if size * 4 < n {
                    n = (size * 4) & 0xFFFFFFFC;
                }
                self.read_block32(current_addr, data[offset..offset + n / 4])?;
                size -= n / 4;
                offset += n;
            }
            Ok()
        } else {
            Err(AccessPortError::MemoryNotAligned)
        }
    }
}

impl AccessPort for MemoryAccessPort {
    fn get_access_port(&self) -> AccessPortNumber {
        self.access_port
    }
    fn set_access_port(&mut self, access_port: AccessPortNumber) {
        self.access_port = access_port;
    }
}

impl NewAccessPort for MemoryAccessPort {
    fn new(access_port: AccessPortNumber) -> Box<AccessPort> {
        Box::new(MemoryAccessPort {
            access_port
        })
    }
}