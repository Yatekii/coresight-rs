use crate::access_ports::memory_access_port::MemoryAccessPort;
use crate::debug_port::DebugPortError;
use crate::debug_port::DebugPort;

pub type AccessPortNumber = u8;

pub enum AccessPortError {
    DebugPortError(DebugPortError),
    InvalidAccessPortNumber,
}

impl AccessPortError {
    pub fn from(error: DebugPortError) -> AccessPortError {
        AccessPortError::DebugPortError(error)
    }
}

pub trait AccessPort {
    const APSEL_SHIFT: u8 = 24;
    const AP_IDR: u8 = 0xFC;

    // AP IDR bitfields:
    // [31:28] Revision
    // [27:24] JEP106 continuation (0x4 for ARM)
    // [23:17] JEP106 vendor ID (0x3B for ARM)
    // [16:13] Class (0b1000=Mem-AP)
    // [12:8]  Reserved
    // [7:4]   AP Variant (non-zero for JTAG-AP)
    // [3:0]   AP Type
    const AP_IDR_REVISION_MASK: u32 = 0xf0000000;
    const AP_IDR_REVISION_SHIFT: u8 = 28;
    const AP_IDR_JEP106_MASK: u32 = 0x0ffe0000;
    const AP_IDR_JEP106_SHIFT: u8 = 17;
    const AP_IDR_CLASS_MASK: u32 = 0x0001e000;
    const AP_IDR_CLASS_SHIFT: u8 = 13;
    const AP_IDR_VARIANT_MASK: u32 = 0x000000f0;
    const AP_IDR_VARIANT_SHIFT: u8 = 4;
    const AP_IDR_TYPE_MASK: u32 = 0x0000000f;

    // MEM-AP type constants
    const AP_TYPE_AHB: u8 = 0x1;
    const AP_TYPE_APB: u8 = 0x2;
    const AP_TYPE_AXI: u8 = 0x4;
    const AP_TYPE_AHB5: u8 = 0x5;

    fn get_access_port(&self) -> AccessPortNumber;
    fn Set_access_port(&mut self, access_port: AccessPortNumber);

    fn new(access_port: AccessPortNumber) -> Box<AccessPort>;

    /// Determine if an AP exists with the given AP number.
    fn access_port_is_valid(debug_port: &mut impl DebugPort, access_port: AccessPortNumber) -> Result<bool, DebugPortError> {
        let idr = debug_port.read_ap(((access_port as u32) << AccessPort::APSEL_SHIFT) | AccessPort::AP_IDR as u32)?;
        Ok(idr != 0)
    }
    
    /// Determines the type of the AP by examining the IDR value and creates a new
    /// AP object of the appropriate class. See #AP_TYPE_MAP for the mapping of IDR
    /// fields to class.
    fn create(debug_port: &mut impl DebugPort, access_port: AccessPortNumber) -> Result<Box<AccessPort>, AccessPortError>  {
        // Attempt to read the IDR for this APSEL. If we get a zero back then there is
        // no AP present, so we return None.
        let idr = debug_port.read_ap(((access_port as u32) << AccessPort::APSEL_SHIFT) | AccessPort::AP_IDR as u32)
                            .map_err(|e| AccessPortError::from(e))?;
        if idr == 0 {
            return Err(AccessPortError::InvalidAccessPortNumber);
        }
        
        // Extract IDR fields used for lookup.
        let designer = (idr & AccessPort::AP_IDR_JEP106_MASK) >> AccessPort::AP_IDR_JEP106_SHIFT;
        let ap_class = (idr & AccessPort::AP_IDR_CLASS_MASK) >> AccessPort::AP_IDR_CLASS_SHIFT;
        let variant = (idr & AccessPort::AP_IDR_VARIANT_MASK) >> AccessPort::AP_IDR_VARIANT_SHIFT;
        let ap_type = (idr & AccessPort::AP_IDR_TYPE_MASK) as u8;

        // Get the AccessPort class to instantiate.        
        return match (variant, ap_type) {
            (0, AccessPort::AP_TYPE_AHB) => Ok(MemoryAccessPort::new(access_port)),
            (0, AccessPort::AP_TYPE_AHB) => Ok(MemoryAccessPort::new(access_port)),
            (0, AccessPort::AP_TYPE_AHB) => Ok(MemoryAccessPort::new(access_port)),
            (0, AccessPort::AP_TYPE_AHB) => Ok(MemoryAccessPort::new(access_port)),
            (0, AccessPort::AP_TYPE_AHB) => Ok(MemoryAccessPort::new(access_port)),
            (0, AccessPort::AP_TYPE_APB) => Ok(MemoryAccessPort::new(access_port)),
            (0, AccessPort::AP_TYPE_AXI) => Ok(MemoryAccessPort::new(access_port)),
            (0, AccessPort::AP_TYPE_AHB5) => Ok(MemoryAccessPort::new(access_port)),
        }
    }

    fn read_reg(&self, debug_port: &mut impl DebugPort, addr: u32) -> Result<u32, DebugPortError> {
        debug_port.read_ap(((self.get_access_port() as u32) << AccessPort::APSEL_SHIFT) | addr)
    }

    fn write_reg(&self, debug_port: &mut impl DebugPort, addr: u32, data: u32) -> Result<(), DebugPortError> {
        debug_port.write_ap(((self.get_access_port() as u32) << AccessPort::APSEL_SHIFT) | addr, data)
    }
}